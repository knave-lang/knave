use std::rc::Rc;

use super::project_modules::ProjectModules;
use crate::core::{BindingId, CoreModule, Import, ImportedName, WrittenName, WrittenUse};
use crate::expand::imported_members::{ImportedMembers, MemberTables};
use crate::modules::ModuleRegistry;
use crate::scope::{ScopeSet, VisibleNames};
use crate::source::SourceMap;

pub(in crate::expand) struct UsedImports<'e> {
    names: VisibleNames<'e>,
    members: MemberTables<'e>,
    modules: &'e ModuleRegistry,
}

impl<'e> UsedImports<'e> {
    pub(in crate::expand) fn new(
        names: VisibleNames<'e>,
        members: MemberTables<'e>,
        modules: &'e ModuleRegistry,
    ) -> Self {
        Self {
            names,
            members,
            modules,
        }
    }

    pub(in crate::expand) fn read_each(
        &self,
        modules: &mut [CoreModule],
        project: &ProjectModules,
        sources: &SourceMap,
    ) {
        for module in modules {
            if !project.contains(module.id) || module.lint_facts.written_uses().is_empty() {
                continue;
            }
            let reader = ImportReader {
                used: self,
                scopes: self.modules.get(module.id).scopes().set(),
                sources,
            };
            module
                .lint_facts
                .read_uses(|use_form| reader.import(use_form));
        }
    }

    fn is_used(
        &self,
        mut entries: impl Iterator<Item = (Rc<str>, BindingId)>,
        scopes: &ScopeSet,
    ) -> bool {
        entries.any(|(name, binding)| self.names.use_of(name, scopes, binding) != Some(false))
    }
}

struct ImportReader<'r, 'e> {
    used: &'r UsedImports<'e>,
    scopes: ScopeSet,
    sources: &'r SourceMap,
}

impl ImportReader<'_, '_> {
    fn import(&self, written: WrittenUse) -> Import {
        let target = self.used.modules.get(written.module);
        let members = ImportedMembers::new(target, self.used.members);
        let mut names = Vec::with_capacity(written.names.len());
        let mut unread_members = Vec::new();
        for (position, name) in written.names.into_vec().into_iter().enumerate() {
            let (name, unread) = self.name(name, &members);
            unread_members.extend(unread.into_iter().map(|member| (position, member)));
            names.push(name);
        }
        Import {
            span: written.span,
            names: names.into_boxed_slice(),
            listed: written.listed,
            module_read: target.status().is_read_by_path(&self.scopes),
            unread_members: unread_members.into_boxed_slice(),
        }
    }

    fn name(
        &self,
        written: WrittenName,
        members: &ImportedMembers,
    ) -> (ImportedName, Vec<Rc<str>>) {
        let text: Rc<str> = written.text(self.sources).into();
        let member_entries = members.of(&text, written.bindings);
        let own = written
            .bindings
            .iter()
            .map(|binding| (Rc::clone(&text), binding));
        let entries = own.chain(member_entries.iter().cloned());
        let used = self.used.is_used(entries, &self.scopes);
        let unread = if used {
            Vec::new()
        } else {
            member_entries.into_iter().map(|(name, _)| name).collect()
        };
        (ImportedName { written, used }, unread)
    }
}

#[cfg(test)]
mod tests {
    use crate::expand::expanded_module::ExpandedModule;

    struct UsedImportsFixture;

    impl UsedImportsFixture {
        fn used(text: &str) -> Vec<Vec<bool>> {
            let program = ExpandedModule::of(text);
            let imports = program.entry_module().lint_facts.imports();
            imports
                .iter()
                .map(|import| import.names.iter().map(|name| name.used).collect())
                .collect()
        }
    }

    #[test]
    fn a_name_the_program_reads_is_used_and_one_it_never_reads_is_not() {
        let used = UsedImportsFixture::used(
            "(use code symbol value-eq)\n(defmacro named (form) (symbol \"x\"))",
        );
        assert_eq!(used, [[true, false]]);
    }

    #[test]
    fn a_use_that_a_macro_writes_records_no_import() {
        let used = UsedImportsFixture::used(
            "(use code list)\n(defmacro imports () (list (quote use) (quote code) (quote first)))\n\
             (imports)",
        );
        assert_eq!(used, [[true]]);
    }

    #[test]
    fn a_use_whose_names_the_macro_writes_records_no_import() {
        let used = UsedImportsFixture::used(
            "(use code list)\n(defmacro imp (p) (list (quote use) p (quote first)))\n\
             (imp code)\n(defn main () (println 1))",
        );
        assert_eq!(used, [[true]]);
    }

    #[test]
    fn a_use_whose_names_the_caller_writes_records_no_import() {
        let used = UsedImportsFixture::used(
            "(use code list)\n(defmacro imp (n) (list (quote use) (quote code) n))\n\
             (imp first)\n(defn main () (println 1))",
        );
        assert_eq!(used, [[true]]);
    }

    #[test]
    fn a_use_whose_head_the_caller_writes_and_path_the_macro_writes_records_no_import() {
        let used = UsedImportsFixture::used(
            "(use code list)\n(defmacro imp (head name) (list head (quote code) name))\n\
             (imp use first)\n(defn main () (println 1))",
        );
        assert_eq!(used, [[true]]);
    }

    #[test]
    fn a_name_a_macro_appends_to_a_written_use_is_not_recorded() {
        let used = UsedImportsFixture::used(
            "(use code concat list)\n\
             (defmacro add-first (form) (concat form (list (quote first))))\n\
             (add-first (use code value-eq))\n(defn main () (println 1))",
        );
        assert_eq!(used, [vec![true, true], vec![false]]);
    }

    #[test]
    fn a_name_a_macro_builds_from_a_caller_anchor_is_not_recorded() {
        let used = UsedImportsFixture::used(
            "(use code concat list symbol)\n\
             (defmacro u (form anchor) (concat form (list (symbol anchor \"first\"))))\n\
             (u (use code) here)\n(defn main () (println 1))",
        );
        assert_eq!(used, [vec![true, true, true], vec![]]);
    }

    #[test]
    fn a_name_a_macro_builds_beside_a_written_name_leaves_the_written_one() {
        let used = UsedImportsFixture::used(
            "(use code concat list symbol)\n\
             (defmacro u (form anchor) (concat form (list (symbol anchor \"first\"))))\n\
             (u (use code value-eq) here)\n(defn main () (println 1))",
        );
        assert_eq!(used, [vec![true, true, true], vec![false]]);
    }
}
