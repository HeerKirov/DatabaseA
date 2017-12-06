use std::collections::HashMap;
use std::borrow::Borrow;
use std::iter::Iterator;
use super::super::analyse::dfa::{DfaWord};
use super::structures::{Switch, Syntax, EmptySyntax, DeleteSyntax, Expression};
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

pub struct DeleteTree{
    nodeset: HashMap<String, Box<DfaNode>>,
    error: (i32, EnumError)
}
impl DeleteTree {
    pub fn new() -> Self {
        Self {
            nodeset: hmap![
                "Start" => NodeStart{},
                "From" => NodeFrom{},
                "SetName" => NodeSetName{},
                "Where" => NodeWhere{}
            ],
            error: (0, EnumError::None)
        }
    }
    pub fn construct(&mut self, li:&[DfaWord]) -> DeleteSyntax {
        let mut table_name = "".to_string();
        let mut wheres = Expression::empty();

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
        DeleteSyntax {
            table_name: table_name,
            wheres: wheres
        }
    }
    pub fn get_error(&self) -> &(i32, EnumError) {
        &self.error
    }
}

//= 节点 ===================================
struct NodeStart;
impl DfaNode for NodeStart {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref kword) if kword == "from" => {
                guide = "From";
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

struct NodeFrom;
impl DfaNode for NodeFrom {
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
                    guide: "SetName".to_string(),
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
            guide: "SetName".to_string(),
            action: vec!["where".to_string(), begin.to_string(), w.len().to_string()],
            error: EnumError::None
        }
    }
    fn allow_array(&self) -> bool {true}
}