use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::lambda_parameters::LambdaParameters;
use super::moved_place::MovedPlace;
use super::moved_value_problems::{CopyBound, MoveSpans, MovedValueProblems};
use super::moved_values::{FieldPath, MoveSite, MovedValues};
use super::mutation::ChangedValues;
use super::problems::OwnershipProblems;
use super::read_usage::ReadUsage;
use super::repeats::{NextPass, RepeatingBody};
use super::stored_definitions::StoredDefinitions;
use super::update_bases::UpdateBases;
use super::update_order::StructUpdateOrder;
use crate::core::{BindingArena, BindingId, FieldLabel};
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::{
    Borrow, ClosureKind, CopyRules, FieldPlace, GenericId, GenericParam, InferredCopies, Pipelines,
    StructTable, TemplateBorrows, Type, TypedArgument, TypedBlock, TypedBodyItem, TypedControl,
    TypedExpr, TypedExprKind, TypedLambda, TypedLeaf, TypedMatch, TypedStructExpr,
};

struct MovedUse<'u> {
    place: &'u FieldPlace<'u>,
    span: Span,
    site: &'u MoveSite,
}

impl MovedUse<'_> {
    fn spans(&self) -> MoveSpans {
        let (used, moved) = (self.span, self.site.span());
        MoveSpans { used, moved }
    }
}

#[derive(Default)]
pub struct MoveReport {
    pub diagnostics: Vec<Diagnostic>,
    pub inferred_copies: InferredCopies,
}

pub struct MoveChecker<'a> {
    copy: CopyRules<'a>,
    structs: &'a StructTable,
    bindings: &'a BindingArena,
    template_borrows: &'a TemplateBorrows,
    generics: &'a FxHashMap<GenericId, GenericParam>,
    pipelines: Pipelines<'a>,
    stored_definitions: &'a StoredDefinitions,
    changed: ChangedValues,
    moved: MovedValues,
    repeating: Option<RepeatingBody>,
    moved_by_earlier_pass: MovedValues,
    copy_functions: FxHashSet<BindingId>,
    once_functions: FxHashSet<BindingId>,
    reported: FxHashSet<BindingId>,
    parameters: LambdaParameters,
    bound_values: FxHashSet<Span>,
    inferred_copies: InferredCopies,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> MoveChecker<'a> {
    pub fn new(
        copy: CopyRules<'a>,
        structs: &'a StructTable,
        bindings: &'a BindingArena,
        template_borrows: &'a TemplateBorrows,
        generics: &'a FxHashMap<GenericId, GenericParam>,
        pipelines: Pipelines<'a>,
        stored_definitions: &'a StoredDefinitions,
    ) -> Self {
        Self {
            copy,
            structs,
            bindings,
            template_borrows,
            generics,
            pipelines,
            stored_definitions,
            changed: ChangedValues::default(),
            moved: MovedValues::default(),
            repeating: None,
            moved_by_earlier_pass: MovedValues::default(),
            copy_functions: FxHashSet::default(),
            once_functions: FxHashSet::default(),
            reported: FxHashSet::default(),
            parameters: LambdaParameters::default(),
            bound_values: FxHashSet::default(),
            inferred_copies: InferredCopies::default(),
            diagnostics: Vec::new(),
        }
    }

    pub fn check(mut self, enclosing: BindingId, lambda: &TypedLambda) -> MoveReport {
        self.parameters = LambdaParameters::of(lambda);
        self.changed = ChangedValues::of(lambda, enclosing);
        self.block(&lambda.body, ReadUsage::Move);
        self.report()
    }

    pub fn check_value(mut self, value: &TypedExpr) -> MoveReport {
        self.expr(value, ReadUsage::Move);
        self.report()
    }

    fn report(self) -> MoveReport {
        MoveReport {
            diagnostics: self.diagnostics,
            inferred_copies: self.inferred_copies,
        }
    }

    fn expr(&mut self, expr: &TypedExpr, usage: ReadUsage) {
        match &expr.kind {
            TypedExprKind::Leaf(leaf) => self.leaf(leaf, expr, usage),
            TypedExprKind::Call(callee, arguments) => self.call(callee, arguments),
            TypedExprKind::Control(control) => self.control(control, usage),
            TypedExprKind::Struct(structure) => self.structure(expr, structure, usage),
            other => self.nested_expr(expr, other),
        }
    }

    fn nested_expr(&mut self, expr: &TypedExpr, kind: &TypedExprKind) {
        match kind {
            TypedExprKind::Recur(arguments) => self.recur(arguments),
            TypedExprKind::Variant(_, arguments, _) => self.expressions(arguments),
            TypedExprKind::Assoc(value, keys, item) => self.assoc(value, keys, item),
            TypedExprKind::TraitObject(value, _) => self.expr(value, ReadUsage::Move),
            _ => self.value_kind(expr, kind),
        }
    }

    fn value_kind(&mut self, expr: &TypedExpr, kind: &TypedExprKind) {
        match kind {
            TypedExprKind::Try { .. } | TypedExprKind::Borrow(..) | TypedExprKind::Deref(..) => {
                self.written(expr)
            }
            TypedExprKind::Fn(lambda) => self.closure(lambda),
            _ => unreachable!("the caller matches a nested expression"),
        }
    }

    fn expressions(&mut self, arguments: &[TypedExpr]) {
        arguments
            .iter()
            .for_each(|argument| self.expr(argument, ReadUsage::Move));
    }

    fn assoc(&mut self, value: &TypedExpr, keys: &[TypedExpr], item: &TypedExpr) {
        self.expr(value, ReadUsage::Move);
        self.expressions(keys);
        self.expr(item, ReadUsage::Move);
    }

    fn leaf(&mut self, leaf: &TypedLeaf, expr: &TypedExpr, usage: ReadUsage) {
        match leaf {
            TypedLeaf::Local(binding) => self.local(*binding, &expr.ty, expr.span, usage),
            TypedLeaf::Global(binding) if usage == ReadUsage::Move => {
                self.stored_definition(*binding, expr)
            }
            _ => {}
        }
    }

    fn stored_definition(&mut self, binding: BindingId, expr: &TypedExpr) {
        let moves_stored_value = self.stored_definitions.contains(binding)
            && !self.copy.is_copy(&expr.ty)
            && !matches!(expr.ty, Type::Function(..));
        if !moves_stored_value {
            return;
        }
        let name = self.bindings.get(binding).name();
        self.diagnostics
            .push(OwnershipProblems::moved_definition(name, expr.span));
    }

    fn written(&mut self, expr: &TypedExpr) {
        match &expr.kind {
            TypedExprKind::Borrow(_, value) | TypedExprKind::Deref(value) => {
                self.expr(value, ReadUsage::Borrow)
            }
            TypedExprKind::Try { value, .. } => self.expr(value, ReadUsage::Move),
            _ => unreachable!("the caller matches a written form"),
        }
    }

    fn structure(&mut self, expr: &TypedExpr, structure: &TypedStructExpr, usage: ReadUsage) {
        match structure {
            TypedStructExpr::Field(owner, _) => self.field_read(expr, owner, usage),
            TypedStructExpr::New(_, fields) => self.field_values(fields),
            TypedStructExpr::Update(_, base, fields) => self.update(expr, base, fields),
        }
    }

    fn update(&mut self, update: &TypedExpr, base: &TypedExpr, fields: &[(FieldLabel, TypedExpr)]) {
        match StructUpdateOrder::of(base) {
            StructUpdateOrder::FieldsBeforePlace(place) => {
                self.field_values(fields);
                let bases = UpdateBases::new(self.structs, &self.copy);
                let unlisted = bases.unlisted_fields(update, fields);
                let moved = bases.moved_fields(&unlisted);
                self.unlisted_read(&place, Self::root_span(base), &unlisted);
                self.move_fields(&place, base.span, moved);
            }
            StructUpdateOrder::BaseFirst => {
                self.expr(base, ReadUsage::Move);
                self.field_values(fields);
            }
        }
    }

    fn field_values(&mut self, fields: &[(FieldLabel, TypedExpr)]) {
        fields
            .iter()
            .for_each(|(_, value)| self.expr(value, ReadUsage::Move));
    }

    fn field_read(&mut self, read: &TypedExpr, owner: &TypedExpr, usage: ReadUsage) {
        let Some(place) = read.field_place() else {
            return self.expr(owner, ReadUsage::Borrow);
        };
        self.place_read(&place, Self::root_span(read));
        if usage.moves_field(read, &self.copy) {
            self.move_field(&place, read.span);
        }
    }

    fn place_read(&mut self, place: &FieldPlace, span: Span) {
        if let Some(site) = self
            .moved
            .site_overlapping(place.root, Self::path_names(place))
        {
            self.use_after_move(place, span, &site);
        }
    }

    fn unlisted_read(&mut self, place: &FieldPlace, span: Span, unlisted: &[(Rc<str>, Type)]) {
        let site = unlisted.iter().find_map(|(field, _)| {
            let field_path = Self::path_names(place).chain(std::iter::once(&**field));
            self.moved.site_overlapping(place.root, field_path)
        });
        if let Some(site) = site {
            self.use_after_move(place, span, &site);
        }
    }

    fn path_names<'p>(place: &'p FieldPlace) -> impl Iterator<Item = &'p str> + Clone {
        place.path.iter().map(|label| &*label.name)
    }

    fn root_span(read: &TypedExpr) -> Span {
        match &read.kind {
            TypedExprKind::Struct(TypedStructExpr::Field(owner, _)) => Self::root_span(owner),
            _ => read.span,
        }
    }

    fn move_fields(&mut self, place: &FieldPlace, span: Span, moving: Vec<Rc<str>>) {
        let moved = MovedPlace::of(place, self.bindings).owned();
        let site = MoveSite::Field { span, place: moved };
        for field in moving {
            let mut path = Self::field_path(place);
            path.push(field);
            self.moved.move_field(place.root, path, site.clone());
        }
    }

    fn move_field(&mut self, place: &FieldPlace, span: Span) {
        if self.copy.is_copy(place.root_type) {
            return;
        }
        let moved =
            MovedPlace::of(place, self.bindings).owned_unless_bound(&self.bound_values, span);
        let site = MoveSite::Field { span, place: moved };
        self.moved
            .move_field(place.root, Self::field_path(place), site);
    }

    fn field_path(place: &FieldPlace) -> FieldPath {
        place.path.iter().map(|label| label.name.clone()).collect()
    }

    fn control(&mut self, control: &TypedControl, usage: ReadUsage) {
        match control {
            TypedControl::Block(block) => self.block(block, usage),
            TypedControl::If(test, then, otherwise) => self.branches(test, then, otherwise, usage),
            TypedControl::Match(matched) => {
                self.expr(&matched.value, self.scrutinee_usage(matched));
                let before = self.moved.clone();
                let mut arm_moves = Vec::new();
                for arm in &matched.arms {
                    self.moved.clone_from(&before);
                    self.block(&arm.body, usage);
                    arm_moves.push(self.moved.clone());
                }
                self.moved = before;
                for moved in arm_moves {
                    self.moved.merge(moved);
                }
            }
        }
    }

    fn scrutinee_usage(&self, matched: &TypedMatch) -> ReadUsage {
        let binds_owned_part = matched
            .arms
            .iter()
            .flat_map(|arm| arm.pattern.binders())
            .any(|binder| !self.copy.is_copy(&binder.ty));
        if binds_owned_part {
            ReadUsage::Move
        } else {
            ReadUsage::Borrow
        }
    }

    fn closure(&mut self, lambda: &TypedLambda) {
        if let Some(next) = self.pass(lambda).next_pass(lambda) {
            self.repeat(lambda, next);
        }
    }

    fn pass(&mut self, lambda: &TypedLambda) -> RepeatingBody {
        let enclosing = self.repeating.replace(RepeatingBody::enter(&self.moved));
        self.block(&lambda.body, ReadUsage::Move);
        std::mem::replace(&mut self.repeating, enclosing).expect("the pass entered a body")
    }

    fn repeat(&mut self, lambda: &TypedLambda, next: NextPass) {
        self.moved_by_earlier_pass.merge(next.carried().clone());
        let after_first_pass = std::mem::replace(&mut self.moved, next.moved_at_start());
        self.pass(lambda);
        self.moved.merge(after_first_pass);
    }

    fn bind(&mut self, binding: BindingId, value: &TypedExpr) {
        match self.bound_function_kind(value) {
            Some(ClosureKind::Fn) => self.copy_functions.insert(binding),
            Some(ClosureKind::FnOnce) => self.once_functions.insert(binding),
            _ => false,
        };
    }

    fn bound_function_kind(&mut self, value: &TypedExpr) -> Option<ClosureKind> {
        match &value.kind {
            TypedExprKind::Fn(lambda) => {
                self.closure(lambda);
                Some(lambda.kind)
            }
            TypedExprKind::Leaf(TypedLeaf::Global(_)) | TypedExprKind::Leaf(TypedLeaf::Rust(_)) => {
                matches!(value.ty, Type::Function(..)).then_some(ClosureKind::Fn)
            }
            TypedExprKind::Leaf(TypedLeaf::Local(other)) => {
                let kind = self.local_function_kind(*other);
                self.expr(value, ReadUsage::Move);
                kind
            }
            TypedExprKind::Call(..) => {
                self.expr(value, ReadUsage::Move);
                Self::once_call_result(&value.ty)
            }
            _ => {
                self.bound_values.insert(value.span);
                self.expr(value, ReadUsage::Move);
                None
            }
        }
    }

    fn once_call_result(ty: &Type) -> Option<ClosureKind> {
        match ty {
            Type::Function(ClosureKind::FnOnce, _, _) => Some(ClosureKind::FnOnce),
            _ => None,
        }
    }

    fn local_function_kind(&self, binding: BindingId) -> Option<ClosureKind> {
        if self.copy_functions.contains(&binding) {
            Some(ClosureKind::Fn)
        } else if self.once_functions.contains(&binding) {
            Some(ClosureKind::FnOnce)
        } else {
            None
        }
    }

    fn call(&mut self, callee: &TypedExpr, arguments: &[TypedArgument]) {
        if !matches!(
            callee.kind,
            TypedExprKind::Leaf(TypedLeaf::Global(_)) | TypedExprKind::Leaf(TypedLeaf::Rust(_))
        ) {
            let usage = self.callee_usage(callee);
            self.expr(callee, usage);
        }
        for (position, argument) in arguments.iter().enumerate() {
            let usage = if self.moves(callee, position, argument) {
                ReadUsage::Move
            } else {
                ReadUsage::Borrow
            };
            self.expr(&argument.value, usage);
        }
    }

    fn callee_usage(&self, callee: &TypedExpr) -> ReadUsage {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding))
                if self.once_functions.contains(binding) =>
            {
                ReadUsage::Move
            }
            _ => ReadUsage::Borrow,
        }
    }

    fn moves(&self, callee: &TypedExpr, position: usize, argument: &TypedArgument) -> bool {
        argument.borrow == Borrow::None
            && !matches!(argument.value.ty, Type::Ref(_, _))
            && !self.template_borrows.keeps(callee, position)
    }

    fn recur(&mut self, arguments: &[TypedArgument]) {
        for argument in arguments {
            let usage = if argument.borrow == Borrow::None
                && !matches!(argument.value.ty, Type::Ref(_, _))
            {
                ReadUsage::Move
            } else {
                ReadUsage::Borrow
            };
            self.expr(&argument.value, usage);
        }
        if let Some(body) = &mut self.repeating {
            body.carry(&self.moved);
        }
    }

    fn branches(
        &mut self,
        test: &TypedExpr,
        then: &TypedExpr,
        otherwise: &TypedExpr,
        usage: ReadUsage,
    ) {
        self.expr(test, ReadUsage::Borrow);
        let before = self.moved.clone();
        self.expr(then, usage);
        let after_then = std::mem::replace(&mut self.moved, before);
        self.expr(otherwise, usage);
        self.moved.merge(after_then);
    }

    fn block(&mut self, block: &TypedBlock, usage: ReadUsage) {
        let last = block.items.len().checked_sub(1);
        for (position, item) in block.items.iter().enumerate() {
            match item {
                TypedBodyItem::Let(binder, value) => self.bind(binder.binding, value),
                TypedBodyItem::Expr(expr) if Some(position) == last => self.expr(expr, usage),
                TypedBodyItem::Expr(expr) => self.expr(expr, ReadUsage::Move),
            }
        }
    }

    fn local(&mut self, binding: BindingId, ty: &Type, span: Span, usage: ReadUsage) {
        match self.moved.site(binding) {
            Some(site) => self.use_after_move(&Self::whole_place(binding, ty), span, &site),
            None => self.record_use(binding, ty, span, usage),
        }
    }

    fn whole_place(root: BindingId, root_type: &Type) -> FieldPlace<'_> {
        FieldPlace {
            root,
            root_type,
            path: Vec::new(),
        }
    }

    fn use_after_move(&mut self, place: &FieldPlace, span: Span, site: &MoveSite) {
        let binding = place.root;
        if self.records_inferred_copy(binding, place.root_type) || !self.reported.insert(binding) {
            return;
        }
        let moved_use = MovedUse { place, span, site };
        let diagnostic = self.use_after_move_diagnostic(&moved_use);
        self.diagnostics.push(diagnostic);
    }

    fn use_after_move_diagnostic(&self, moved_use: &MovedUse) -> Diagnostic {
        let moved = self.moved_value(moved_use);
        if let Some(diagnostic) = self.bound_help(moved_use, &moved) {
            return diagnostic;
        }
        let place = moved_use.place;
        self.ownership_help(place.root, place.root_type, &moved)
            .unwrap_or_else(|| self.reuse_help(moved, moved_use))
    }

    fn reuse_help(&self, moved: Diagnostic, moved_use: &MovedUse) -> Diagnostic {
        let name = self.bindings.get(moved_use.place.root).name();
        if self.pipelines.is_pipeline(moved_use.place.root_type) {
            return OwnershipProblems::pipeline_help(moved, name);
        }
        let site = moved_use.site;
        MovedValueProblems::reuse_suggestions(moved, site, name)
    }

    fn moved_value(&self, moved_use: &MovedUse) -> Diagnostic {
        let site = moved_use.site;
        let moved_place = site.moved_place(self.bindings.get(moved_use.place.root).name());
        if self.moved_in_earlier_pass(moved_use) {
            MovedValueProblems::use_after_earlier_pass_move(moved_place, moved_use.spans())
        } else {
            MovedValueProblems::use_after_move(moved_place, moved_use.spans())
        }
    }

    fn moved_in_earlier_pass(&self, moved_use: &MovedUse) -> bool {
        let place = moved_use.place;
        self.moved_by_earlier_pass
            .site_overlapping(place.root, Self::path_names(place))
            .is_some_and(|earlier| earlier.span() == moved_use.site.span())
    }

    fn bound_help(&self, moved_use: &MovedUse, moved: &Diagnostic) -> Option<Diagnostic> {
        let place = moved_use.place;
        if matches!(place.root_type, Type::Function(..)) {
            return Some(OwnershipProblems::closure_move_help(moved.clone()));
        }
        let Some(Type::Generic(generic)) = place.pair_part_type() else {
            return None;
        };
        if !self.parameters.contains(place.root) {
            return None;
        }
        let parameter = self.generics.get(generic)?;
        let bound = CopyBound {
            parameter: &parameter.name,
            copy_trait: self.copy.copy_trait_name(self.bindings),
        };
        let name = self.bindings.get(place.root).name();
        Some(MovedValueProblems::missing_copy_bound(
            moved_use.site.moved_place(name),
            &bound,
            moved_use.spans(),
        ))
    }

    fn ownership_help(
        &self,
        binding: BindingId,
        ty: &Type,
        moved: &Diagnostic,
    ) -> Option<Diagnostic> {
        if matches!(ty, Type::Mut(_, _)) {
            return Some(OwnershipProblems::mut_parameter_move_help(moved.clone()));
        }
        if self.changed.is_changed(binding) {
            return Some(OwnershipProblems::changed_value_help(moved.clone()));
        }
        if self.parameters.contains(binding) && self.changed.is_recursive_argument(binding) {
            return Some(OwnershipProblems::recursive_parameter_help(moved.clone()));
        }
        None
    }

    fn record_use(&mut self, binding: BindingId, ty: &Type, span: Span, usage: ReadUsage) {
        let is_copy = self.copy.is_copy(ty) || self.copy_functions.contains(&binding);
        if usage == ReadUsage::Move && !is_copy {
            self.moved.move_whole(binding, span);
        }
    }

    fn records_inferred_copy(&mut self, binding: BindingId, ty: &Type) -> bool {
        let copies_parameter =
            self.parameters.is_unwritten(binding) && matches!(ty, Type::Generic(_));
        if copies_parameter {
            self.inferred_copies.insert(binding);
        }
        copies_parameter
    }
}
