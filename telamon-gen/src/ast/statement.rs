use ast::*;
use ir;

/// A toplevel definition or constraint.
#[derive(Debug)]
pub struct EnumDef {
    pub name: String,
    pub doc: Option<String>,
    pub variables: Vec<VarDef>,
    pub statements: Vec<EnumStatement>
}

impl PartialEq for EnumDef {
    fn eq(&self, rhs: &Self) -> bool {
        self.name == rhs.name
    }
}

#[derive(Debug)]
pub struct TriggerDef {
    pub foralls: Vec<VarDef>,
    pub conditions: Vec<Condition>,
    pub code: String,
}

#[derive(Debug)]
pub struct CounterDef {
    pub name: RcStr,
    pub doc: Option<String>,
    pub visibility: ir::CounterVisibility,
    pub vars: Vec<VarDef>,
    pub body: CounterBody,
}

impl PartialEq for CounterDef {
    fn eq(&self, rhs: &Self) -> bool {
        self.name == rhs.name
    }
}

#[derive(Debug, PartialEq)]
pub enum ChoiceDef {
    CounterDef(CounterDef),
    EnumDef(EnumDef),
}

#[derive(Debug)]
pub struct SetDef {
    pub name: String,
    pub doc: Option<String>,
    pub arg: Option<VarDef>,
    pub superset: Option<SetRef>,
    pub disjoint: Vec<String>,
    pub keys: Vec<(ir::SetDefKey, Option<VarDef>, String)>,
    pub quotient: Option<Quotient>,
}
