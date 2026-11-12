use crate::value_kind::ValueKind;

pub struct LocalScopes {
    scopes: Vec<Vec<Local>>,
}

struct Local {
    name: &'static str,
    kind: ValueKind,
}

impl LocalScopes {
    pub fn new() -> Self {
        Self {
            scopes: vec![Vec::new()],
        }
    }

    pub fn open(&mut self) {
        self.scopes.push(Vec::new());
    }

    pub fn close(&mut self) {
        if self.depth() > 0 {
            self.scopes.pop();
        }
    }

    pub fn depth(&self) -> usize {
        self.scopes.len() - 1
    }

    pub fn bind(&mut self, name: &'static str, kind: ValueKind) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.push(Local { name, kind });
        }
    }

    pub fn visible(&self, kind: ValueKind) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self
            .scopes
            .iter()
            .flatten()
            .filter(|local| local.kind == kind)
            .map(|local| local.name)
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::LocalScopes;
    use crate::value_kind::ValueKind;

    #[test]
    fn a_closed_scope_hides_its_locals_and_the_root_never_closes() {
        let mut scopes = LocalScopes::new();
        scopes.bind("v", ValueKind::Numbers);
        scopes.open();
        scopes.bind("w", ValueKind::Numbers);
        assert_eq!(scopes.visible(ValueKind::Numbers), ["v", "w"]);
        scopes.close();
        scopes.close();
        assert_eq!(scopes.visible(ValueKind::Numbers), ["v"]);
        assert_eq!(scopes.depth(), 0);
    }
}
