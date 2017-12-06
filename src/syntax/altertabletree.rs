use std::collections::HashMap;
use std::borrow::Borrow;
use std::iter::Iterator;
use super::super::analyse::dfa::{DfaWord};
use super::structures::{
    Switch, Syntax, EmptySyntax, AlterTableSyntax, Expression,
    TableFieldSyntax, TableForeignSyntax
};
use super::trees::{DfaNode, AResult, EnumError, EnumResult, Tree};

macro_rules! hmap {
( $( $x:expr => $y:expr ),* ) => {
    {
        let mut temp_map:HashMap<String, Box<DfaNode>> = HashMap::new();
        $(
            temp_map.insert($x.to_string(), Box::new($y));
        )*
        temp_map
    }
};
}

fn dfa_name(d:&DfaWord) -> String {
    match d {
            &DfaWord::Kword(ref k) => format!("Kword:{:?}", &k),
            &DfaWord::Var(ref v) => format!("Var:{:?}", &v),
            &DfaWord::Integer(ref int) => format!("Int:{:?}", &int),
            &DfaWord::Float(ref flo) => format!("Float:{:?}", &flo),
            &DfaWord::Str(ref s) => format!("String:{:?}", &s),
            &DfaWord::Signal(ref si) => format!("Signal:{:?}", &si),
            &DfaWord::Bool(b) => format!("Bool:{:?}", b),
            &DfaWord::End => format!("End:()")
    }
}

pub struct AlterTableTree {
    nodeset: HashMap<String, Box<DfaNode>>,
    error: (i32, EnumError)
}
impl AlterTableTree {
    pub fn new() -> Self {
        Self {
            nodeset: hmap![
                "Start" => NodeStart{},
                "SetTableName" => NodeSetTableName{},
                "AddField" => NodeAddField{},
                "AlterField" => NodeAlterField{},
                "SetField" => NodeSetField{},
                "SetType" => NodeSetType{},
                "SetTypeParam" => NodeSetTypeParam{},
                "TypeParam" => NodeTypeParam{},
                "SetPrimary" => NodeSetPrimary{},
                "SetDefault" => NodeSetDefault{},
                "DropField" => NodeDropField{},
                "SetField2" => NodeSetField2{}
            ],
            error: (0, EnumError::None)
        }
    }
    pub fn construct(&mut self, li:&[DfaWord]) -> AlterTableSyntax {
        let mut name = "".to_string();
        let mut adds = Vec::new();
        let mut alters = Vec::new();
        let mut drops = Vec::new();

        let mut new = TableFieldSyntax::empty();
        let mut last = "";

        let mut node:&DfaNode = self.nodeset["Start"].borrow();
        let mut i = 0;
        while i < li.len() {
            let res:AResult;
            //print!("[{}][{}]", i, dfa_name(&li[i]));
            if node.allow_array() {
                let mut end_i:i32 = i as i32;
                res = node.analysis_array(&li, i as i32, &mut end_i);
                i = end_i as usize;
                //print!("->[{}]", i);
            } else {
                res = node.analysis(&li[i]);
            }
            //println!("result={},guide={}, e={}", res.result as i32, res.guide, res.error as i32);
            if res.error != EnumError::None {
                self.error = (i as i32, res.error);
                break;
            }else {
                if !node.allow_array() {
                    match res.result {
                        EnumResult::Return => {}
                        _ => {i+=1;}
                    }
                }
                if res.action.len() > 0 {
                    match &res.action[0][..] {
                       "tablename" => {
                           name = res.action[1].to_string();
                       },
                       "addfield" => {
                           last = "add";
                           new.name = res.action[1].to_string();
                       },
                       "alterfield" => {
                           last = "alter";
                           new.name = res.action[1].to_string();
                       },
                       "dropfield" => {
                           drops.push(res.action[1].to_string());
                       }
                       "fieldtype" => {
                           new.t = res.action[1].to_string();
                           if new.t.as_str() == "str" {new.t = "str:0".to_string();}
                       },
                       "fieldtypeparam" => {
                           let v = res.action[1].to_string();
                           if new.t.starts_with("str") {
                               new.t = format!("str:{}", v);
                           }
                       },
                       "complete" => {
                           //提交一个新field。
                           if last == "add" {adds.push(new);}
                           else if last == "alter" {alters.push(new);}
                           new = TableFieldSyntax::empty();
                       },
                       "primary" => {new.primary = true;},
                       "unique" => {new.unique = true;},
                       "notnull" => {new.not_null = true;},
                       "auto_inc" => {new.auto_inc = true;},
                       "default" => {new.default = Option::Some(res.action[1].to_string());},
                       _ => {}
                    }
                }
                if res.guide != "" {
                    node = self.nodeset[&res.guide].borrow();
                }
            }  
        }
        if last == "add" {adds.push(new);}else if last == "alter" {alters.push(new);}
        //println!("add={}, alter={}, drop={}", adds.len(), alters.len(), drops.len());
        AlterTableSyntax {
            name: name,
            adds: adds,
            alters: alters,
            drops: drops
        }
    }
    pub fn get_error(&self) -> &(i32, EnumError) {
        &self.error
    }
}

//= 节点 =======================================
struct NodeStart;
impl DfaNode for NodeStart {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref var) => {
                guide = "SetTableName";
                action = vec!["tablename".to_string(), var.to_string()];
            },
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeSetTableName;
impl DfaNode for NodeSetTableName {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref s) if s == "add" => {
                guide = "AddField";
            },
            &DfaWord::Kword(ref s) if s == "alter" => {
                guide = "AlterField";
            },
            &DfaWord::Kword(ref s) if s == "drop" => {
                guide = "DropField";
            }
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeAddField;
impl DfaNode for NodeAddField {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref var) => {
                guide = "SetField";
                action = vec!["addfield".to_string(), var.to_string()];
            },
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeAlterField;
impl DfaNode for NodeAlterField {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref var) => {
                guide = "SetField";
                action = vec!["alterfield".to_string(), var.to_string()];
            },
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeSetField;
impl DfaNode for NodeSetField {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) if k == "integer" || k == "float" || k == "str" || k == "bool" => {
                guide = "SetType";
                action = vec!["fieldtype".to_string(), k.to_string()];
            },
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeSetType;
impl DfaNode for NodeSetType {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) => match k.as_str() {
                "unique" => {action = vec!["unique".to_string()]},
                "not_null" => {action = vec!["notnull".to_string()]},
                "auto_increment" => {action = vec!["auto_inc".to_string()]},
                "primary" => {guide = "SetPrimary"},
                "default" => {guide = "SetDefault"},
                _ => {
                    guide = "SetTableName";
                    action = vec!["complete".to_string()];
                    result = EnumResult::Return;
                }
            },
            &DfaWord::Signal(ref s) if s == "(" => {
                guide = "SetTypeParam";
            },
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeSetTypeParam;
impl DfaNode for NodeSetTypeParam {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Integer(i) => {
                guide = "TypeParam";
                action = vec!["fieldtypeparam".to_string(), i.to_string()];
            },
            &DfaWord::Float(f) => {
                guide = "TypeParam";
                action = vec!["fieldtypeparam".to_string(), f.to_string()];
            },
            &DfaWord::Str(ref s) => {
                guide = "TypeParam";
                action = vec!["fieldtypeparam".to_string(), s.to_string()];
            },
            &DfaWord::Bool(b) => {
                guide = "TypeParam";
                action = vec!["fieldtypeparam".to_string(), b.to_string()];
            },
            &DfaWord::Signal(ref s) if s == ")" => {
                guide = "SetType";
            },
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeTypeParam;
impl DfaNode for NodeTypeParam {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Signal(ref s) if s == "," => {
                guide = "SetTypeParam";
            },
            &DfaWord::Signal(ref s) if s == ")" => {
                guide = "SetTypeParam";
                result = EnumResult::Return;
            }
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeSetPrimary;
impl DfaNode for NodeSetPrimary {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) if k == "key" => {
                guide = "SetType";
                action = vec!["primary".to_string()];
            },
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeSetDefault;
impl DfaNode for NodeSetDefault {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Integer(i) => {
                guide = "SetType";
                action = vec!["default".to_string(), i.to_string()];
            },
            &DfaWord::Float(f) => {
                guide = "SetType";
                action = vec!["default".to_string(), f.to_string()];
            },
            &DfaWord::Str(ref s) => {
                guide = "SetType";
                action = vec!["default".to_string(), s.to_string()];
            },
            &DfaWord::Bool(b) => {
                guide = "SetType";
                action = vec!["default".to_string(), b.to_string()];
            },
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeDropField;
impl DfaNode for NodeDropField {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref var) => {
                guide = "SetField2";
                action = vec!["dropfield".to_string(), var.to_string()];
            },
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeSetField2;
impl DfaNode for NodeSetField2 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(_) => {
                guide = "SetTableName";
                result = EnumResult::Return;
            },
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}