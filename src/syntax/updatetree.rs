use std::collections::HashMap;
use std::borrow::Borrow;
use std::iter::Iterator;
use super::super::analyse::dfa::{DfaWord};
use super::structures::{Switch, Syntax, EmptySyntax, UpdateSyntax, Expression};
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

pub struct UpdateTree {
    nodeset: HashMap<String, Box<DfaNode>>,
    error: (i32, EnumError)
}
impl UpdateTree {
    pub fn new() -> Self {
        Self {
            nodeset: hmap![
                "Start" => NodeStart{},
                "SetName" => NodeSetName{},
                "Set" => NodeSet{},
                "Column" => NodeColumn{},
                "Equals" => NodeEquals{},
                "Value" => NodeValue{},
                "Where" => NodeWhere{}
            ],
            error: (0, EnumError::None)
        }
    }
}
impl UpdateTree {
    pub fn construct(&mut self, li:&[DfaWord]) -> UpdateSyntax {
        let mut table_name = "".to_string();
        let mut wheres = Expression::empty();
        let mut sets = HashMap::new();
        let mut last_column = "".to_string();

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
                       "name" => {
                           table_name = res.action[1].to_string();
                       },
                       "column" => {
                           last_column = res.action[1].to_string();
                       },   
                       "value" => {
                           if last_column != "".to_string() {
                                let v = res.action[2].to_string();
                                sets.insert(last_column.to_string(), match res.action[1].as_str() {
                                    "str" => DfaWord::Str(v.to_string()),
                                    "integer" => DfaWord::Integer(v.parse().unwrap()),
                                    "float" => DfaWord::Float(v.parse().unwrap()),
                                    "bool" => DfaWord::Bool(v.parse().unwrap()),
                                    _ => panic!("Wrong DfaWord Type.")
                                });
                           }
                           last_column = "".to_string();
                       },
                       "where" => {
                            let begin_i:usize = res.action[1].parse().unwrap();
                            let end_i:usize = res.action[2].parse().unwrap();
                            // 调用expression的构造。
                            wheres = Expression::new(&li[begin_i..end_i]);
                        },
                       _ =>{}
                    }
                }
                if res.guide != "" {
                    node = self.nodeset[&res.guide].borrow();
                }
            }
        }
        UpdateSyntax {
            table_name: table_name,
            sets: sets,
            wheres: wheres
        }
    }
    pub fn get_error(&self) -> &(i32, EnumError) {
        &self.error
    }
}

//= 节点 ===============================
struct NodeStart;
impl DfaNode for NodeStart {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref var) => {
                guide = "SetName";
                action = vec!["name".to_string(), var.to_string()];
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

struct NodeSetName;
impl DfaNode for NodeSetName {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref var) if var == "set" => {
                guide = "Set";
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

struct NodeSet;
impl DfaNode for NodeSet {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref var) => {
                guide = "Column";
                action = vec!["column".to_string(), var.to_string()];
            },
            &DfaWord::Kword(ref kword) if kword == "where" => {
                guide = "Where";
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

struct NodeColumn;
impl DfaNode for NodeColumn {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Signal(ref s) if s == "=" => {
                guide = "Equals";
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

struct NodeEquals;
impl DfaNode for NodeEquals {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Str(ref s) => {
               guide = "Value";
               action = vec!["value".to_string(), "str".to_string(), s.to_string()];
           },
           &DfaWord::Integer(i) => {
               guide = "Value";
               action = vec!["value".to_string(), "integer".to_string(), i.to_string()];
           },
           &DfaWord::Float(f) => {
               guide = "Value";
               action = vec!["value".to_string(), "float".to_string(), f.to_string()];
           },
           &DfaWord::Bool(b) => {
               guide = "Value";
               action = vec!["value".to_string(), "bool".to_string(), b.to_string()];
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

struct NodeValue;
impl DfaNode for NodeValue {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref kword)=> {
                guide = "Set";
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

struct NodeWhere;
impl DfaNode for NodeWhere {
    fn analysis(&self, w:&DfaWord) -> AResult {
        panic!("Not Allowed");
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        let mut i = begin as usize;
        while i < w.len() {
            if let DfaWord::Kword(_) = w[i] {
                *end = i as i32;
                return AResult {
                    result: EnumResult::Return,
                    guide: "Set".to_string(),
                    error: EnumError::None,
                    action: vec!["where".to_string(), begin.to_string(), i.to_string()]
                }
            }else{
                i+=1;
            }
        }
        *end = w.len() as i32;
        AResult {
            result: EnumResult::Accept,
            guide: "Set".to_string(),
            action: vec!["where".to_string(), begin.to_string(), w.len().to_string()],
            error: EnumError::None
        }
    }
    fn allow_array(&self) -> bool {true}
}