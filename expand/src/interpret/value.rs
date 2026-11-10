use std::rc::Rc;

use super::CodeList;
use super::code_list::{ListTooLarge, PassedLimit};
use crate::core::{BindingId, Builtin, Keyword, Lambda};
use crate::reader::{Form, FormKind, NumberValue, Symbol};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Value {
    Form(Form),
    List(CodeList),
    Bool(bool),
    Unit,
    Closure(Rc<Closure>),
    Builtin(Builtin),
}

impl Value {
    pub fn describe(&self) -> &'static str {
        match self {
            Value::Form(form) => Self::describe_form(form),
            Value::List(_) => "a list",
            Value::Bool(_) => "a boolean",
            Value::Unit => "unit",
            Value::Closure(_) | Value::Builtin(_) => "a function",
        }
    }

    fn describe_form(form: &Form) -> &'static str {
        match form.kind() {
            FormKind::Symbol(symbol) if Keyword::field_name(symbol.name()).is_some() => "a keyword",
            FormKind::Symbol(_) => "a symbol",
            FormKind::String(_) => "a string",
            FormKind::Number(number) => match number.value() {
                NumberValue::Integer(_) => "an integer",
                NumberValue::Float(_) => "a float",
            },
            FormKind::List(_) => "a list",
        }
    }

    pub fn as_form(&self) -> Option<&Form> {
        match self {
            Value::Form(form) => Some(form),
            _ => None,
        }
    }

    pub fn to_form(&self) -> Result<Option<Form>, ListTooLarge> {
        match self {
            Value::Form(form) => Ok(Some(form.clone())),
            Value::List(list) => list.to_form().map(Some),
            _ => Ok(None),
        }
    }

    pub fn to_form_within(
        &self,
        allowance: &mut usize,
        enclosing_lists: usize,
    ) -> Result<Form, PassedLimit> {
        match self {
            Value::List(list) => list.to_form_within(allowance, enclosing_lists),
            _ => {
                *allowance = allowance.checked_sub(1).ok_or(PassedLimit::FormCount)?;
                self.as_form().cloned().ok_or(PassedLimit::FormCount)
            }
        }
    }

    pub fn is_code(&self) -> bool {
        matches!(self, Value::Form(_) | Value::List(_))
    }

    pub fn is_list(&self) -> bool {
        self.as_code_list().is_some()
    }

    pub fn as_code_list(&self) -> Option<CodeList> {
        match self {
            Value::Form(form) => CodeList::of_form(form),
            Value::List(list) => Some(list.clone()),
            _ => None,
        }
    }

    pub fn keyword_name(&self) -> Option<&str> {
        self.as_form()
            .and_then(Form::as_symbol)
            .and_then(|symbol| Keyword::field_name(symbol.name()))
    }

    pub fn keyword_field(&self) -> Option<Form> {
        let form = self.as_form()?;
        let keyword = form.as_symbol()?;
        let field_name = Keyword::field_name(keyword.name())?;
        let field = Symbol::with_scopes(field_name, keyword.scopes().clone());
        Some(Form::new(FormKind::Symbol(field), form.span()))
    }

    pub fn keyword_trim_end(&self) -> Option<Form> {
        let form = self.as_form()?;
        let keyword = form.as_symbol()?;
        let trimmed = Keyword::trim_end(keyword.name())?;
        let trimmed = Symbol::with_scopes(trimmed, keyword.scopes().clone());
        Some(Form::new(FormKind::Symbol(trimmed), form.span()))
    }

    pub fn field_keyword(&self) -> Option<Form> {
        let form = self.as_form()?;
        let field = form.as_symbol()?;
        let keyword_name = Keyword::of_field(field.name())?;
        let keyword = Symbol::with_scopes(keyword_name, field.scopes().clone());
        Some(Form::new(FormKind::Symbol(keyword), form.span()))
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Closure {
    pub lambda: Rc<Lambda>,
    pub env: Env,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Env(Option<Rc<Frame>>);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Frame {
    binding: BindingId,
    value: Value,
    parent: Env,
}

impl Drop for Frame {
    fn drop(&mut self) {
        if self.owns_a_closure() || self.parent.is_a_sole_chain() {
            ChainRelease::default().release_links_of(self);
        }
    }
}

impl Frame {
    fn owns_a_closure(&self) -> bool {
        matches!(&self.value, Value::Closure(closure) if Rc::strong_count(closure) == 1)
    }

    fn take_closure_env(&mut self) -> Option<Env> {
        match std::mem::replace(&mut self.value, Value::Unit) {
            Value::Closure(closure) => Rc::try_unwrap(closure).ok().map(|closure| closure.env),
            _ => None,
        }
    }
}

#[derive(Default)]
struct ChainRelease {
    closure_envs: Vec<Env>,
}

impl ChainRelease {
    fn release_links_of(mut self, frame: &mut Frame) {
        let mut link = self.next_link_of(frame);
        loop {
            link = match link.into_sole_frame() {
                Some(mut linked) => self.next_link_of(&mut linked),
                None => match self.closure_envs.pop() {
                    Some(env) => env,
                    None => return,
                },
            };
        }
    }

    fn next_link_of(&mut self, frame: &mut Frame) -> Env {
        if frame.owns_a_closure() {
            self.closure_envs.extend(frame.take_closure_env());
        }
        std::mem::take(&mut frame.parent)
    }
}

impl Env {
    pub fn bind(&self, binding: BindingId, value: Value) -> Env {
        Env(Some(Rc::new(Frame {
            binding,
            value,
            parent: self.clone(),
        })))
    }

    pub fn frame_count(&self) -> usize {
        std::iter::successors(self.0.as_deref(), |frame| frame.parent.0.as_deref()).count()
    }

    fn is_sole_owner(&self) -> bool {
        self.0
            .as_ref()
            .is_some_and(|frame| Rc::strong_count(frame) == 1)
    }

    fn is_a_sole_chain(&self) -> bool {
        self.0
            .as_ref()
            .is_some_and(|frame| Rc::strong_count(frame) == 1 && frame.parent.is_sole_owner())
    }

    fn into_sole_frame(self) -> Option<Frame> {
        self.0.and_then(|frame| Rc::try_unwrap(frame).ok())
    }

    pub fn lookup(&self, binding: BindingId) -> Option<&Value> {
        let mut current = self.0.as_deref();
        while let Some(frame) = current {
            if frame.binding == binding {
                return Some(&frame.value);
            }
            current = frame.parent.0.as_deref();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::{Closure, Env, Value};
    use crate::core::{BindingId, Lambda};

    struct ChainFixture;

    impl ChainFixture {
        const LONG_CHAIN_LINKS: usize = 1_000_000;

        fn binding() -> BindingId {
            BindingId::from_index(0)
        }

        fn closure_over(env: Env) -> Rc<Closure> {
            let lambda = Lambda::new(Vec::new(), Default::default());
            Rc::new(Closure {
                lambda: Rc::new(lambda),
                env,
            })
        }

        fn closure_chain(links: usize) -> Value {
            (0..links).fold(Value::Unit, |previous, _| {
                let env = Env::default().bind(Self::binding(), previous);
                Value::Closure(Self::closure_over(env))
            })
        }

        fn parent_chain(links: usize) -> Env {
            (0..links).fold(Env::default(), |env, _| {
                env.bind(Self::binding(), Value::Unit)
            })
        }

        fn two_branches(links: usize) -> Env {
            let closure = Value::Closure(Self::closure_over(Self::parent_chain(links)));
            Self::parent_chain(links).bind(Self::binding(), closure)
        }
    }

    #[test]
    fn a_long_closure_chain_frees_without_recursion() {
        drop(ChainFixture::closure_chain(ChainFixture::LONG_CHAIN_LINKS));
    }

    #[test]
    fn a_long_parent_chain_frees_without_recursion() {
        drop(ChainFixture::parent_chain(ChainFixture::LONG_CHAIN_LINKS));
    }

    #[test]
    fn a_frame_with_two_long_branches_frees_without_recursion() {
        drop(ChainFixture::two_branches(ChainFixture::LONG_CHAIN_LINKS));
    }

    #[test]
    fn freeing_a_frame_keeps_a_closure_held_elsewhere() {
        let kept = ChainFixture::closure_over(Env::default());
        let env = Env::default().bind(ChainFixture::binding(), Value::Closure(Rc::clone(&kept)));
        drop(env);
        assert_eq!(Rc::strong_count(&kept), 1);
    }

    #[test]
    fn freeing_a_frame_keeps_a_parent_held_elsewhere() {
        let parent = Env::default().bind(ChainFixture::binding(), Value::Bool(true));
        drop(parent.bind(ChainFixture::binding(), Value::Unit));
        assert!(matches!(
            parent.lookup(ChainFixture::binding()),
            Some(Value::Bool(true))
        ));
    }

    #[test]
    fn freeing_a_chain_frees_a_closure_it_owned() {
        let held = ChainFixture::closure_over(Env::default());
        let inner = Env::default().bind(ChainFixture::binding(), Value::Closure(Rc::clone(&held)));
        let closure = ChainFixture::closure_over(inner);
        drop(Env::default().bind(ChainFixture::binding(), Value::Closure(closure)));
        assert_eq!(Rc::strong_count(&held), 1);
    }
}
