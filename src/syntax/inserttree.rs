use std::collections::HashMap;
use std::borrow::Borrow;
use std::iter::Iterator;
use super::super::analyse::dfa::{DfaWord};
use super::structures::{Switch, Syntax, EmptySyntax, InsertSyntax, Expression};
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

pub struct InsertTree {
    nodeset: HashMap<String, Box<DfaNode>>,
    error: (i32, EnumError)
}
impl InsertTree {
    pub fn new() -> Self {
        let mut map:HashMap<String, Box<DfaNode>> = hmap![
            "Start" => NodeStart{},
            "SetName" => NodeSetName{},
            "SetColumn" => NodeSetColumn{},
            "ColumnName" => NodeColumnName{},
            "Values" => NodeValues{},
            "SetValue" => NodeSetValue{},
            "Value" => NodeValue{}
        ];
        Self {
            nodeset: map,
            error: (0, EnumError::None)
        }
    }
}
impl InsertTree {
    pub fn construct(&mut self, li:&[DfaWord]) -> InsertSyntax {
        let mut table_name = "".to_string();
        let mut columns = Vec::new();
        let mut values = Vec::new();
        let mut has_head = true;

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
                       "setcolumn" => {
                           columns.push(res.action[1].to_string());
                       },
                       "setvalue" => {
                           let v = res.action[2].to_string();
                           match res.action[1].as_str() {
                               "str" => values.push(DfaWord::Str(v.to_string())),
                               "integer" => values.push(DfaWord::Integer(v.parse().unwrap())),
                               "float" => values.push(DfaWord::Float(v.parse().unwrap())),
                               "bool" => values.push(DfaWord::Bool(v.parse().unwrap())),
                               _ => panic!("Wrong DfaWord Type.")
                           }
                       },
                       _ =>{}
                    }
                }
                if res.guide != "" {
                    node = self.nodeset[&res.guide].borrow();
                }
            }
        }
        let map:HashMap<String, DfaWord> = if columns.len() > 0 && columns.len() != values.len() {
            self.error = (1, EnumError::SyntaxError);
            HashMap::new()
        }else if columns.len() > 0 {
            let mut i = 0;
            let mut map = HashMap::new();
            while i < columns.len() {
                map.insert(columns[i].to_string(), values[i].copy());
                i += 1;
            }
            map
        }else{
            has_head = false;
            let mut map = HashMap::new();
            for (i, j) in values.iter().enumerate() {
                map.insert(i.to_string(), j.copy());
            }
            map
        };

        InsertSyntax {
            table_name: table_name.to_string(),
            has_head: has_head,
            values: vec![map]
        }
    }
    pub fn get_error(&self) -> &(i32, EnumError) {
        &self.error
    }
}

//= 节点 =================================
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
            &DfaWord::Kword(ref kword) if kword == "values" => {
                guide = "Values";
            },
            &DfaWord::Signal(ref s) if s == "(" => {
                guide = "SetColumn";
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

struct NodeSetColumn;
impl DfaNode for NodeSetColumn {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref var) => {
                action = vec!["setcolumn".to_string(), var.to_string()];
                guide = "ColumnName";
            },
            &DfaWord::Signal(ref s) if s == ")" => {
                guide = "SetName";
            }
            _ => {error = EnumError::SyntaxError;}
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

struct NodeColumnName;
impl DfaNode for NodeColumnName {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
           &DfaWord::Signal(ref s) => {
               if s == "," {
                   guide = "SetColumn";
               }else if s == ")" {
                   guide = "SetColumn";
                   result = EnumResult::Return;
               }
           },
           _ => {EnumError::SyntaxError;}
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

struct NodeValues;
impl DfaNode for NodeValues {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
           &DfaWord::Signal(ref s) if s == "(" => {
               guide = "SetValue";
           },
           _ => {error = EnumError::SyntaxError;}
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

struct NodeSetValue;
impl DfaNode for NodeSetValue {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
           &DfaWord::Str(ref s) => {
               guide = "Value";
               action = vec!["setvalue".to_string(), "str".to_string(), s.to_string()];
           },
           &DfaWord::Integer(i) => {
               guide = "Value";
               action = vec!["setvalue".to_string(), "integer".to_string(), i.to_string()];
           },
           &DfaWord::Float(f) => {
               guide = "Value";
               action = vec!["setvalue".to_string(), "float".to_string(), f.to_string()];
           },
           &DfaWord::Bool(b) => {
               guide = "Value";
               action = vec!["setvalue".to_string(), "bool".to_string(), b.to_string()];
           },
           &DfaWord::Signal(ref s) if s == ")" => {
               guide = "Values";
           }
           _ => {error = EnumError::SyntaxError;}
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
           &DfaWord::Signal(ref s) => {
               if s == "," {
                   guide = "SetValue";
               }else if s == ")" {
                   guide = "SetValue";
                   result = EnumResult::Return;
               }else {
                   error = EnumError::IllegalSignal;
               }
           },
           _ => {error = EnumError::SyntaxError;}
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
