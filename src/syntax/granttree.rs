use std::collections::HashMap;
use std::borrow::Borrow;
use std::iter::Iterator;
use super::super::analyse::dfa::{DfaWord};
use super::structures::{Switch, Syntax, EmptySyntax, GrantSyntax, Expression};
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

pub struct GrantTree {
    nodeset: HashMap<String, Box<DfaNode>>,
    error: (i32, EnumError),
    is_grant: bool
}
impl GrantTree {
    pub fn new(is_grant: bool) -> Self {
        Self {
            nodeset: hmap![
                "Start" => NodeStart{},
                "SetPrivilege" => NodeSetPrivilege{},
                "PrivilegeCheck1" => NodePrivilegeCheck1{},
                "PrivilegeCheck2" => NodePrivilegeCheck2{},
                "PrivilegeCheck3" => NodePrivilegeCheck3{},
                "PrivilegeCheck4" => NodePrivilegeCheck4{},
                "SetAllPrivileges" => NodeSetAllPrivileges{},
                "On" => NodeOn{},
                "OnTable" => NodeOnTable{},
                "OnDatabase" => NodeOnDatabase{},
                "OnView" => NodeOnView{},
                "SetObject" => NodeSetObject{},
                "User" => NodeUser{},
                "SetUser" => NodeSetUser{}
            ],
            error: (0, EnumError::None),
            is_grant: is_grant
        }
    }
    pub fn construct(&mut self, li:&[DfaWord]) -> GrantSyntax {
        let mut users = Vec::new();
        let mut grantall = false;
        let mut grants = Vec::new();
        let mut objects = Vec::new();

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
                       "grantall" => {
                           grantall = true;
                       },
                       "grant" => {
                           grants.push(res.action[1].to_string());
                       },
                       "ontable" => {
                           objects.push(("table".to_string(), res.action[1].to_string()));
                       },
                       "ondatabase" => {
                           objects.push(("database".to_string(), res.action[1].to_string()))
                       },
                       "onview" => {
                           objects.push(("view".to_string(), res.action[1].to_string()))
                       }
                       "user" => {
                           users.push(res.action[1].to_string());
                       },
                       _ =>{}
                    }
                }
                if res.guide != "" {
                    node = self.nodeset[&res.guide].borrow();
                }
            }
        }
        GrantSyntax {
            all: grantall,
            grants: grants,
            users: users,
            objects: objects,
            is_grant: self.is_grant
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
            &DfaWord::Kword(ref k) if k=="select"||k=="insert"||k=="update"||k=="delete"||k=="help" => {
                guide = "SetPrivilege";
                action = vec!["grant".to_string(), k.to_string()];
            },
            &DfaWord::Kword(ref k) if k=="create" => {
                guide = "PrivilegeCheck1";
            },
            &DfaWord::Kword(ref k) if k=="alter" => {
                guide = "PrivilegeCheck2";
            },
            &DfaWord::Kword(ref k) if k=="drop" => {
                guide = "PrivilegeCheck3";
            },
            &DfaWord::Kword(ref k) if k=="all" => {
                guide = "PrivilegeCheck4";
            },
            &DfaWord::Kword(ref k) if k == "on" => {
                guide = "On";
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

struct NodeSetPrivilege;
impl DfaNode for NodeSetPrivilege {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) => {
                guide = "Start";
                result = EnumResult::Return;
            },
            &DfaWord::Signal(ref s) if s == "," => {
                guide = "Start";
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

struct NodePrivilegeCheck1;
impl DfaNode for NodePrivilegeCheck1 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) if k == "table"||k == "view" => {
                guide = "SetPrivilege";
                action = vec!["grant".to_string(), format!("create{}", k)];
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

struct NodePrivilegeCheck2;
impl DfaNode for NodePrivilegeCheck2 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) if k == "table"||k == "view" => {
                guide = "SetPrivilege";
                action = vec!["grant".to_string(), format!("alter{}", k)];
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

struct NodePrivilegeCheck3;
impl DfaNode for NodePrivilegeCheck3 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) if k == "table"||k == "view" => {
                guide = "SetPrivilege";
                action = vec!["grant".to_string(), format!("drop{}", k)];
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

struct NodePrivilegeCheck4;
impl DfaNode for NodePrivilegeCheck4 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) if k == "privileges" => {
                guide = "SetAllPrivileges";
                action = vec!["grantall".to_string()];
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

struct NodeSetAllPrivileges;
impl DfaNode for NodeSetAllPrivileges {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) if k == "on" => {
                guide = "On";
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

struct NodeOn;
impl DfaNode for NodeOn {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref k) if k == "table" => {
                guide = "OnTable";
            },
            &DfaWord::Kword(ref k) if k == "database" => {
                guide = "OnDatabase";
            },
            &DfaWord::Kword(ref k) if k == "view" => {
                guide = "OnView";
            }
            &DfaWord::Kword(ref k) if k=="to"||k=="from" => {
                guide = "User";
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

struct NodeOnTable;
impl DfaNode for NodeOnTable {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref k) => {
                guide = "SetObject";
                action = vec!["ontable".to_string(), k.to_string()];
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

struct NodeOnView;
impl DfaNode for NodeOnView {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref k) => {
                guide = "SetObject";
                action = vec!["onview".to_string(), k.to_string()];
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

struct NodeOnDatabase;
impl DfaNode for NodeOnDatabase {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref k) => {
                guide = "SetObject";
                action = vec!["ondatabase".to_string(), k.to_string()];
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

struct NodeSetObject;
impl DfaNode for NodeSetObject {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(_) => {
                guide = "On";
                result = EnumResult::Return;
            },
            &DfaWord::Signal(ref s) if s == "," => {
                guide = "On";
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

struct NodeUser;
impl DfaNode for NodeUser {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref k) => {
                guide = "SetUser";
                action = vec!["user".to_string(), k.to_string()];
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

struct NodeSetUser;
impl DfaNode for NodeSetUser {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Signal(ref k) if k == "," => {
                guide = "User";
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