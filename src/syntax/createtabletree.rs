use std::collections::HashMap;
use std::borrow::Borrow;
use std::iter::Iterator;
use super::super::analyse::dfa::{DfaWord};
use super::structures::{
    Switch, Syntax, EmptySyntax, CreateTableSyntax, Expression,
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

pub struct CreateTableTree {
    nodeset: HashMap<String, Box<DfaNode>>,
    error: (i32, EnumError)
}
impl CreateTableTree {
    pub fn new() -> Self {
        Self {
            nodeset: hmap![
                "Start" => NodeStart{},
                "SetTableName" => NodeSetTableName{},
                "Fields" => NodeFields{},
                "EndField" => NodeEndField{},
                "SetField" => NodeSetField{},
                "SetType" => NodeSetType{},
                "SetTypeParam" => NodeSetTypeParam{},
                "TypeParam" => NodeTypeParam{},
                "SetPrimary" => NodeSetPrimary{},
                "SetDefault" => NodeSetDefault{},
                "ForeignCheck" => NodeForeignCheck{},
                "SetForeign" => NodeSetForeign{},
                "ForeignSignal1" => NodeForeignSignal1{},
                "ForeignSignal2" => NodeForeignSignal2{},
                "ForeignSignal3" => NodeForeignSignal3{},
                "ForeignSignal4" => NodeForeignSignal4{},
                "ForeignField" => NodeForeignField{},
                "ForeignReference" => NodeForeignReference{},
                "ForeignTable" => NodeForeignTable{},
                "ForeignKey" => NodeForeignKey{}
            ],
            error: (0, EnumError::None)
        }
    }
    pub fn construct(&mut self, li:&[DfaWord]) -> CreateTableSyntax {
        let mut table_name = "".to_string();
        let mut fields = Vec::new();
        let mut foreigns = Vec::new();

        let mut new_field = TableFieldSyntax::empty();
        let mut new_foreign = TableForeignSyntax::empty();

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
                           table_name = res.action[1].to_string();
                       },
                       "fieldname" => {
                           new_field.name = res.action[1].to_string();
                       },
                       "fieldtype" => {
                           new_field.t = res.action[1].to_string();
                           if new_field.t.as_str() == "str" {new_field.t = "str:0".to_string();}
                       },
                       "fieldtypeparam" => {
                           let v = res.action[1].to_string();
                           if new_field.t.starts_with("str") {
                               new_field.t = format!("str:{}", v);
                           }
                       },
                       "completefield" => {
                           //提交一个新field。
                           fields.push(new_field);
                           new_field = TableFieldSyntax::empty();
                       },
                       "primary" => {new_field.primary = true;},
                       "unique" => {new_field.unique = true;},
                       "notnull" => {new_field.not_null = true;},
                       "auto_inc" => {new_field.auto_inc = true;},
                       "default" => {new_field.default = Option::Some(res.action[1].to_string());},
                       "foreign" => {new_foreign.field = res.action[1].to_string();},
                       "foreigntable" => {new_foreign.foreign_table = res.action[1].to_string();},
                       "foreignfield" => {new_foreign.foreign_field = res.action[1].to_string();},
                       "completeforeign" => {
                           foreigns.push(new_foreign);
                           new_foreign = TableForeignSyntax::empty();
                       }
                       _ => {}
                    }
                }
                if res.guide != "" {
                    node = self.nodeset[&res.guide].borrow();
                }
            }  
        }
        CreateTableSyntax {
            name: table_name,
            fields: fields,
            foreigns: foreigns
        }
    }
    pub fn get_error(&self) -> &(i32, EnumError) {
        &self.error
    }
}

//= 节点 =====================================
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
            &DfaWord::Signal(ref s) if s == "(" => {
                guide = "Fields";
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

struct NodeFields;
impl DfaNode for NodeFields {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref var) => {
                guide = "SetField";
                action = vec!["fieldname".to_string(), var.to_string()];
            },
            &DfaWord::Kword(ref k) if k == "foreign" => {
                guide = "ForeignCheck";
            },
            &DfaWord::Signal(ref s) if s == ")" => {
                guide = "EndField";
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

struct NodeEndField;
impl DfaNode for NodeEndField {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
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
                _ => {error = EnumError::SyntaxError;}
            },
            &DfaWord::Signal(ref s) if s == "(" => {
                guide = "SetTypeParam";
            },
            &DfaWord::Signal(ref s) if s == "," => {
                guide = "Fields";
                action = vec!["completefield".to_string()];
            },
            &DfaWord::Signal(ref s) if s == ")" => {
                guide = "Fields";
                action = vec!["completefield".to_string()];
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

struct NodeForeignCheck;
impl DfaNode for NodeForeignCheck {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) if k == "key" => {
                guide = "SetForeign";
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

struct NodeSetForeign;
impl DfaNode for NodeSetForeign {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Signal(ref k) if k == "(" => {
                guide = "ForeignSignal1";
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

struct NodeForeignSignal1;
impl DfaNode for NodeForeignSignal1 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref k) => {
                guide = "ForeignField";
                action = vec!["foreign".to_string(), k.to_string()];
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

struct NodeForeignField;
impl DfaNode for NodeForeignField {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Signal(ref k) if k == ")" => {
                guide = "ForeignSignal2";
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

struct NodeForeignSignal2;
impl DfaNode for NodeForeignSignal2 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) if k == "reference" => {
                guide = "ForeignReference";
            },
            _ => {error = EnumError::SyntaxError;}
        }
        return AResult {result: result, action: action, guide: guide.to_string(), error: error}
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{panic!("Not Allowed");}
    fn allow_array(&self) -> bool {false}
}

struct NodeForeignReference;
impl DfaNode for NodeForeignReference {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref k) => {
                guide = "ForeignTable";
                action = vec!["foreigntable".to_string(), k.to_string()];
            },
            _ => {error = EnumError::SyntaxError;}
        }
        return AResult {result: result, action: action, guide: guide.to_string(), error: error}
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{panic!("Not Allowed");}
    fn allow_array(&self) -> bool {false}
}

struct NodeForeignTable;
impl DfaNode for NodeForeignTable {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Signal(ref k) if k == "(" => {
                guide = "ForeignSignal3";
            },
            _ => {error = EnumError::SyntaxError;}
        }
        return AResult {result: result, action: action, guide: guide.to_string(), error: error}
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{panic!("Not Allowed");}
    fn allow_array(&self) -> bool {false}
}

struct NodeForeignSignal3;
impl DfaNode for NodeForeignSignal3 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref k) => {
                guide = "ForeignKey";
                action = vec!["foreignfield".to_string(), k.to_string()];
            },
            _ => {error = EnumError::SyntaxError;}
        }
        return AResult {result: result, action: action, guide: guide.to_string(), error: error}
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{panic!("Not Allowed");}
    fn allow_array(&self) -> bool {false}
}

struct NodeForeignKey;
impl DfaNode for NodeForeignKey {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Signal(ref k) if k == ")" => {
                guide = "ForeignSignal4";
            },
            _ => {error = EnumError::SyntaxError;}
        }
        return AResult {result: result, action: action, guide: guide.to_string(), error: error}
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{panic!("Not Allowed");}
    fn allow_array(&self) -> bool {false}
}

struct NodeForeignSignal4;
impl DfaNode for NodeForeignSignal4 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Signal(ref k) if k == "," => {
                guide = "Fields";
                action = vec!["completeforeign".to_string()];
            },
            &DfaWord::Signal(ref k) if k == ")" => {
                guide = "Fields";
                action = vec!["completeforeign".to_string()];
                result = EnumResult::Return;
            },
            _ => {error = EnumError::SyntaxError;}
        }
        return AResult {result: result, action: action, guide: guide.to_string(), error: error}
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{panic!("Not Allowed");}
    fn allow_array(&self) -> bool {false}
}