#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinModule {
    Kernel,
    Code,
    PreludeCode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Builtin {
    First,
    Rest,
    List,
    Prepend,
    Concat,
    Len,
    IsEmpty,
    Nth,
    Take,
    Drop,
    IsSymbol,
    IsKeyword,
    IsList,
    IsString,
    IsNumber,
    SymbolName,
    KeywordName,
    KeywordField,
    FieldKeyword,
    KeywordTrimEnd,
    MakeSymbol,
    MakeString,
    SameBinding,
    IsPatternHead,
    DefinitionSymbol,
    TypeDefinitionSymbol,
    DefinitionCall,
    TypeDefinitionCall,
    ExpandCall,
    ClosestName,
    SortedNames,
    DistinctLeadingLines,
    Fail,
    NumberAdd,
    NumberSub,
    NumberLt,
    NumberLe,
    NumberGt,
    NumberGe,
    ValueEq,
    BoolNot,
    EscapeString,
    EscapeChar,
    EscapeFormat,
    StringStripSuffix,
    StringStripPrefix,
    StringLines,
    StringTrimStart,
    StringTrimEnd,
    StringLength,
    StringDrop,
    StringLowercase,
    StringContains,
    AddModuleNote,
    ModuleNotes,
    DeclareDefinition,
    DeclareTypeDefinition,
    DeclareMacroDefinition,
    FormLocation,
    InTasksDirectory,
    PinnedAt,
    ProgramModules,
    ModuleExports,
    ModuleDefinitions,
    CurrentModule,
    ModuleSymbol,
    RequireModule,
    ProjectFileName,
    ReadForm,
    Located,
    ProjectModules,
}

impl Builtin {
    const NAMES: [(Builtin, &'static str); 71] = [
        (Builtin::First, "first"),
        (Builtin::Rest, "rest"),
        (Builtin::List, "list"),
        (Builtin::Prepend, "prepend"),
        (Builtin::Concat, "concat"),
        (Builtin::Len, "len"),
        (Builtin::IsEmpty, "empty?"),
        (Builtin::Nth, "nth"),
        (Builtin::Take, "take"),
        (Builtin::Drop, "drop"),
        (Builtin::IsSymbol, "symbol?"),
        (Builtin::IsKeyword, "keyword?"),
        (Builtin::IsList, "list?"),
        (Builtin::IsString, "string?"),
        (Builtin::IsNumber, "number?"),
        (Builtin::SymbolName, "symbol-name"),
        (Builtin::KeywordName, "keyword-name"),
        (Builtin::KeywordField, "keyword-field"),
        (Builtin::FieldKeyword, "field-keyword"),
        (Builtin::KeywordTrimEnd, "keyword-trim-end"),
        (Builtin::MakeSymbol, "symbol"),
        (Builtin::MakeString, "string"),
        (Builtin::SameBinding, "same-binding?"),
        (Builtin::IsPatternHead, "pattern-head?"),
        (Builtin::DefinitionSymbol, "definition-symbol"),
        (Builtin::TypeDefinitionSymbol, "type-definition-symbol"),
        (Builtin::DefinitionCall, "definition-call"),
        (Builtin::TypeDefinitionCall, "type-definition-call"),
        (Builtin::ExpandCall, "expand-call"),
        (Builtin::ClosestName, "closest-name"),
        (Builtin::SortedNames, "sorted-names"),
        (Builtin::DistinctLeadingLines, "distinct-leading-lines"),
        (Builtin::Fail, "fail"),
        (Builtin::NumberAdd, "number-add"),
        (Builtin::NumberSub, "number-sub"),
        (Builtin::NumberLt, "number-lt"),
        (Builtin::NumberLe, "number-le"),
        (Builtin::NumberGt, "number-gt"),
        (Builtin::NumberGe, "number-ge"),
        (Builtin::ValueEq, "value-eq"),
        (Builtin::BoolNot, "bool-not"),
        (Builtin::EscapeString, "escape-string"),
        (Builtin::EscapeChar, "escape-char"),
        (Builtin::EscapeFormat, "escape-format"),
        (Builtin::StringStripSuffix, "string-strip-suffix"),
        (Builtin::StringStripPrefix, "string-strip-prefix"),
        (Builtin::StringLines, "string-lines"),
        (Builtin::StringTrimStart, "string-trim-start"),
        (Builtin::StringTrimEnd, "string-trim-end"),
        (Builtin::StringLength, "string-length"),
        (Builtin::StringDrop, "string-drop"),
        (Builtin::StringLowercase, "string-lowercase"),
        (Builtin::StringContains, "string-contains?"),
        (Builtin::AddModuleNote, "add-module-note"),
        (Builtin::ModuleNotes, "module-notes"),
        (Builtin::DeclareDefinition, "declare-definition"),
        (Builtin::DeclareTypeDefinition, "declare-type-definition"),
        (Builtin::DeclareMacroDefinition, "declare-macro-definition"),
        (Builtin::FormLocation, "form-location"),
        (Builtin::InTasksDirectory, "in-tasks-directory?"),
        (Builtin::PinnedAt, "pinned-at"),
        (Builtin::ProgramModules, "program-modules"),
        (Builtin::ModuleExports, "module-exports"),
        (Builtin::ModuleDefinitions, "module-definitions"),
        (Builtin::CurrentModule, "current-module"),
        (Builtin::ModuleSymbol, "module-symbol"),
        (Builtin::RequireModule, "require-module"),
        (Builtin::ProjectFileName, "project-file-name"),
        (Builtin::ReadForm, "read-form"),
        (Builtin::Located, "located"),
        (Builtin::ProjectModules, "project-modules"),
    ];

    pub fn module(self) -> BuiltinModule {
        match self {
            Builtin::PinnedAt | Builtin::RequireModule => BuiltinModule::PreludeCode,
            _ => BuiltinModule::Code,
        }
    }

    pub fn all() -> impl Iterator<Item = Builtin> {
        Self::NAMES.iter().map(|(builtin, _)| *builtin)
    }

    pub fn position(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        let (_, name) = Self::NAMES[self.position()];
        name
    }
}

#[cfg(test)]
mod tests {
    use super::Builtin;

    #[test]
    fn each_builtin_finds_its_own_name_row() {
        for builtin in Builtin::all() {
            let (named, _) = Builtin::NAMES[builtin.position()];
            assert_eq!(named, builtin);
        }
    }
}
