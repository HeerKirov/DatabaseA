use std::collections::HashMap;
use std::borrow::Borrow;
use super::super::analyse::dfa::{DfaWord};
use super::structures::{
    Syntax, EmptySyntax, ColSyntax, HelpSyntax,
    UseSyntax, DropTableSyntax, CreateDatabaseSyntax, DropDatabaseSyntax,
    CreateUserSyntax, AlterUserSyntax, DropUserSyntax, GrantSyntax,
    CreateViewSyntax, DropViewSyntax
};
use super::trees::{DfaNode, AResult, EnumError, EnumResult, Tree};
use super::selecttree::{SelectTree};
use super::inserttree::{InsertTree};
use super::updatetree::{UpdateTree};
use super::deletetree::{DeleteTree};
use super::createtabletree::{CreateTableTree};
use super::altertabletree::{AlterTableTree};
use super::granttree::{GrantTree};

//顶层语法树。
pub struct PublicTree {
    nodeset:HashMap<String, Box<DfaNode>>,
    error: (i32, EnumError)
}
impl PublicTree {
    pub fn new() -> Self {
        let mut map:HashMap<String, Box<DfaNode>> = HashMap::new();
        map.insert("Start".to_string(), Box::new(NodeStart{}));
        map.insert("Create".to_string(), Box::new(NodeCreate{}));
        map.insert("Alter".to_string(), Box::new(NodeAlter{}));
        map.insert("Drop".to_string(), Box::new(NodeDrop{}));
        map.insert("Insert".to_string(), Box::new(NodeInsert{}));
        Self {
            nodeset: map,
            error: (0, EnumError::None)
        }
    }
}
impl PublicTree { 
    pub fn construct(&mut self, li:&[DfaWord]) -> ColSyntax {
        // 顶层语法树的作用是向次级语法树转移，因此这里几乎没有什么逻辑代码。
        let mut node:&DfaNode = self.nodeset["Start"].borrow();

        let mut i = 0;
        while i < li.len() {
            let AResult{result, action, guide, error} = node.analysis(&li[i]);
            // println!("[{}]res={}, guide={}, error={}", i, result as i32, guide, error as i32);
            if error != EnumError::None {
                self.error = (i as i32, error);
                break;
            } else {
                match result {
                    EnumResult::Accept => {i+=1;},
                    EnumResult::AcceptAndNo => {i+=1;},
                    EnumResult::Clear => {i+=1;},
                    EnumResult::Return => {}
                }
                if action.len() > 1 {
                    match &action[0][..] {
                        "goto" => {
                            let goto = &action[1][..];
                            // 构造次级语法树，并且返回次级语法树的构造结果。
                            // 查找配对的分号
                            let mut slice_i = i;
                            loop {
                                if slice_i >= li.len() {break;}
                                else if let DfaWord::Signal(ref s) = li[slice_i] {
                                    if s == ";" {
                                        break;
                                    }else {slice_i += 1;}
                                }else {slice_i += 1;}
                            }
                            match goto {
                                "select" => {
                                    let subvec = &li[i..slice_i];
                                    let mut selecttree = SelectTree::new();
                                    let res = selecttree.construct(subvec);
                                    if selecttree.get_error().0 > 0 {
                                        self.error = *selecttree.get_error();
                                        return ColSyntax::None;
                                    }else{return ColSyntax::Select(res);}
                                },
                                "insert" => {
                                    let subvec = &li[i..slice_i];
                                    let mut inserttree = InsertTree::new();
                                    let res = inserttree.construct(subvec);
                                    if inserttree.get_error().0 > 0 {
                                        self.error = *inserttree.get_error();
                                        return ColSyntax::None;
                                    }else{return ColSyntax::Insert(res);}
                                },
                                "update" => {
                                    let subvec = &li[i..slice_i];
                                    let mut updatetree = UpdateTree::new();
                                    let res = updatetree.construct(subvec);
                                    if updatetree.get_error().0 > 0 {
                                        self.error = *updatetree.get_error();
                                        return ColSyntax::None;
                                    }else{return ColSyntax::Update(res);}
                                },
                                "delete" => {
                                    let subvec = &li[i..slice_i];
                                    let mut deletetree = DeleteTree::new();
                                    let res = deletetree.construct(subvec);
                                    if deletetree.get_error().0 > 0 {
                                        self.error = *deletetree.get_error();
                                        return ColSyntax::None;
                                    }else{ return ColSyntax::Delete(res);}
                                }
                                "use" => {
                                    //简单的语法直接在顶级树内处理。
                                    let subvec = &li[i..slice_i];
                                    if subvec.len() <= 0 {
                                        self.error = (0, EnumError::SyntaxError);
                                        return ColSyntax::None;
                                    }else{
                                        if let DfaWord::Var(ref var) = subvec[0] {
                                            return ColSyntax::Use(UseSyntax::new(var));
                                        }else{
                                            self.error = (0, EnumError::SyntaxError);
                                            return ColSyntax::None;
                                        }
                                    }
                                },
                                "help" => {
                                    let subvec = &li[i..slice_i];
                                    let mut params = Vec::new();
                                    for i in subvec.iter() {
                                        if let &DfaWord::Kword(ref k) = i {
                                            params.push(k.to_string());
                                        }else if let &DfaWord::Var(ref v) = i {
                                            params.push(v.to_string());
                                        }
                                    }
                                    return ColSyntax::Help(HelpSyntax{
                                        params: params
                                    })
                                },
                                "createtable" => {
                                    let subvec = &li[i..slice_i];
                                    let mut tree = CreateTableTree::new();
                                    let res = tree.construct(subvec);
                                    if tree.get_error().0 > 0 {
                                        self.error = *tree.get_error();
                                        return ColSyntax::None;
                                    }else{ return ColSyntax::CreateTable(res);}
                                },
                                "altertable" => {
                                    let subvec = &li[i..slice_i];
                                    let mut tree = AlterTableTree::new();
                                    let res = tree.construct(subvec);
                                    if tree.get_error().0 > 0 {
                                        self.error = *tree.get_error();
                                        return ColSyntax::None;
                                    }else{ return ColSyntax::AlterTable(res);}
                                },
                                "droptable" => {
                                    let subvec = &li[i..slice_i];
                                    if subvec.len() <= 0 {
                                        self.error = (0, EnumError::SyntaxError);
                                        return ColSyntax::None;
                                    }else{
                                        if let DfaWord::Var(ref var) = subvec[0] {
                                            return ColSyntax::DropTable(DropTableSyntax::new(var));
                                        }else{
                                            self.error = (1, EnumError::SyntaxError);
                                            return ColSyntax::None;
                                        }
                                    }
                                },
                                "createview" => {
                                    let subvec = &li[i..slice_i];
                                    if subvec.len() < 3 {
                                        self.error = (subvec.len() as i32, EnumError::SyntaxError);
                                        return ColSyntax::None;
                                    }else{
                                        if let DfaWord::Var(ref name) = subvec[0] {
                                            if let DfaWord::Kword(ref k) = subvec[1] {
                                                if k == "as" {
                                                    if let DfaWord::Kword(ref k) = subvec[2] {
                                                        if k == "select" {
                                                            let mut selecttree = SelectTree::new();
                                                            let res = selecttree.construct(&subvec[3..]);
                                                            if selecttree.get_error().0 > 0 {
                                                                self.error = *selecttree.get_error();
                                                                return ColSyntax::None;
                                                            }else{
                                                                return ColSyntax::CreateView(CreateViewSyntax::new(name, res));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                                "dropview" => {
                                    let subvec = &li[i..slice_i];
                                    if subvec.len() <= 0 {
                                        self.error = (subvec.len() as i32, EnumError::SyntaxError);
                                        return ColSyntax::None;
                                    }
                                    if let DfaWord::Var(ref name) = subvec[0] {
                                        return ColSyntax::DropView(DropViewSyntax::new(name));
                                    }
                                },
                                "createdatabase" => {
                                    let subvec = &li[i..slice_i];
                                    if subvec.len() <= 0 {
                                        self.error = (0, EnumError::SyntaxError);
                                        return ColSyntax::None;
                                    }else{
                                        if let DfaWord::Var(ref var) = subvec[0] {
                                            return ColSyntax::CreateDatabase(CreateDatabaseSyntax::new(var));
                                        }else{
                                            self.error = (1, EnumError::SyntaxError);
                                            return ColSyntax::None;
                                        }
                                    }
                                },
                                "dropdatabase" => {
                                    let subvec = &li[i..slice_i];
                                    if subvec.len() <= 0 {
                                        self.error = (0, EnumError::SyntaxError);
                                        return ColSyntax::None;
                                    }else{
                                        if let DfaWord::Var(ref var) = subvec[0] {
                                            return ColSyntax::DropDatabase(DropDatabaseSyntax::new(var));
                                        }else{
                                            self.error = (0, EnumError::SyntaxError);
                                            return ColSyntax::None;
                                        }
                                    }
                                },
                                "createuser" | "createadminuser" => {
                                    let subvec = &li[i..slice_i];
                                    if subvec.len() < 4 {
                                        self.error = (subvec.len() as i32, EnumError::SyntaxError);
                                        return ColSyntax::None;
                                    }else{
                                        let staff = goto == "createadminuser";
                                        if let DfaWord::Var(ref username) = subvec[0] {
                                            if let DfaWord::Kword(ref k) = subvec[1] {
                                                if k == "with" {
                                                    if let DfaWord::Kword(ref s) = subvec[2] {
                                                        if s == "password" {
                                                            if let DfaWord::Str(ref password) = subvec[3] {
                                                                return ColSyntax::CreateUser(CreateUserSyntax::new(username, password, staff));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.error = (1, EnumError::SyntaxError);
                                        return ColSyntax::None;
                                    }
                                },
                                "alteruser" => {
                                    let subvec = &li[i..slice_i];
                                    if subvec.len() < 4 {
                                        self.error = (subvec.len() as i32, EnumError::SyntaxError);
                                        return ColSyntax::None;
                                    }else{
                                        if let DfaWord::Var(ref username) = subvec[0] {
                                            if let DfaWord::Kword(ref k) = subvec[1] {
                                                if k == "with" {
                                                    if let DfaWord::Kword(ref s) = subvec[2] {
                                                        if s == "password" {
                                                            if let DfaWord::Str(ref password) = subvec[3] {
                                                                return ColSyntax::AlterUser(AlterUserSyntax::new(username, password));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.error = (1, EnumError::SyntaxError);
                                        return ColSyntax::None;
                                    }
                                },
                                "dropuser" => {
                                    let subvec = &li[i..slice_i];
                                    if subvec.len() <= 0 {
                                        self.error = (1, EnumError::SyntaxError);
                                        return ColSyntax::None;
                                    }else{
                                        if let DfaWord::Var(ref var) = subvec[0] {
                                            return ColSyntax::DropUser(DropUserSyntax::new(var));
                                        }else{
                                            self.error = (1, EnumError::SyntaxError);
                                            return ColSyntax::None;
                                        }
                                    }
                                },
                                "grant" | "revoke" => {
                                    let subvec = &li[i..slice_i];
                                    let mut tree = GrantTree::new(goto == "grant");
                                    let res = tree.construct(subvec);
                                    if tree.get_error().0 > 0 {
                                        self.error = *tree.get_error();
                                        return ColSyntax::None;
                                    }else{return ColSyntax::Grant(res);}
                                }
                                _ => {
                                    return ColSyntax::None;
                                }
                            }
                        },
                        _ => {}
                    }
                }
                if guide != "" {
                    node = self.nodeset[&guide].borrow();
                }
            }
        }
        return ColSyntax::None;
    }
    pub fn get_error(&self) -> &(i32, EnumError) {
        &self.error
    }

    pub fn get_error_string(&self) -> Option<String> {
        if self.error.0 > 0 {
            Option::Some(match self.error.1 {
                EnumError::None => format!("No Error."),
                EnumError::SyntaxError => format!("Syntax Error in {}.", self.error.0),
                EnumError::IllegalSignal => format!("Illegal Signal in {}.", self.error.0),
                EnumError::UnknownStart => format!("Unexpected Start.")
            })
        }else{
            Option::None
        }
    }
}

//= 节点 =====================================================
struct NodeStart;
impl DfaNode for NodeStart {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut tp = true;
        match w {
            &DfaWord::Kword(ref word) => {
                let mut guide = "";
                let mut error = EnumError::None;
                match &word[..] {
                    "use" | "select" | "update" | "delete" | "help" | "grant" | "revoke" => {tp = false;}
                    "create" => {guide = "Create";},
                    "alter" => {guide = "Alter";},
                    "drop" => {guide = "Drop";},
                    "insert" => {guide = "Insert";},
                    _ => {error = EnumError::SyntaxError;}
                }
                return AResult {
                    result: EnumResult::Accept,
                    action: if tp {vec![]} else {vec!["goto".to_string(), word.to_string()]},
                    guide: guide.to_string(),
                    error: error
                };
            },
            &DfaWord::End => {return AResult {
                result: EnumResult::Accept,
                action: vec![],
                guide: "".to_string(),
                error: EnumError::None
            }},
            _ => {return AResult {
                result: EnumResult::Accept,
                action: vec![],
                guide: "".to_string(),
                error: EnumError::UnknownStart,
            }}
        }
        
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeCreate;
impl DfaNode for NodeCreate {
    fn analysis(&self, w:&DfaWord) -> AResult {
        match w {
           &DfaWord:: Kword(ref word) => match &word[..] {
                "database" | "view" | "table" | "user" | "adminuser" => {return AResult {
                    result: EnumResult::Accept,
                    action: vec!["goto".to_string(), ("create".to_string() + &word[..])],
                    guide: "".to_string(),
                    error: EnumError::None
                }},
                _ => {return AResult {
                    result: EnumResult::Accept,
                    action: vec![],
                    guide: "".to_string(),
                    error: EnumError::SyntaxError
                }}
            },
            _ => {return AResult {
                result: EnumResult::Accept,
                action: vec![],
                guide: "".to_string(),
                error: EnumError::SyntaxError
            }}
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeAlter;
impl DfaNode for NodeAlter {
    fn analysis(&self, w:&DfaWord) -> AResult {
        match w {
           &DfaWord:: Kword(ref word) => match &word[..] {
                "view" | "table" | "user" => {return AResult {
                    result: EnumResult::Accept,
                    action: vec!["goto".to_string(), ("alter".to_string() + &word[..])],
                    guide: "".to_string(),
                    error: EnumError::None
                }},
                _ => {return AResult {
                    result: EnumResult::Accept,
                    action: vec![],
                    guide: "".to_string(),
                    error: EnumError::SyntaxError
                }}
            },
            _ => {return AResult {
                result: EnumResult::Accept,
                action: vec![],
                guide: "".to_string(),
                error: EnumError::SyntaxError
            }}
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeDrop;
impl DfaNode for NodeDrop {
    fn analysis(&self, w:&DfaWord) -> AResult {
        match w {
            &DfaWord::Kword(ref word) => match &word[..] {
                "database" | "view" | "table" | "user" => {return AResult {
                    result: EnumResult::Accept,
                    action: vec!["goto".to_string(), ("drop".to_string() + &word[..])],
                    guide: "".to_string(),
                    error: EnumError::None
                }},
                _ => {return AResult {
                    result: EnumResult::Accept,
                    action: vec![],
                    guide: "".to_string(),
                    error: EnumError::SyntaxError
                }}
            },
            _ => {return AResult {
                result: EnumResult::Accept,
                action: vec![],
                guide: "".to_string(),
                error: EnumError::SyntaxError
            }}
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeInsert;
impl DfaNode for NodeInsert {
    fn analysis(&self, w:&DfaWord) -> AResult {
        match w {
            &DfaWord::Kword(ref word) => match &word[..] {
                "into" => {return AResult {
                    result: EnumResult::Accept,
                    action: vec!["goto".to_string(), "insert".to_string()],
                    guide: "".to_string(),
                    error: EnumError::None
                }},
                _ => {return AResult {
                    result: EnumResult::Accept,
                    action: vec![],
                    guide: "".to_string(),
                    error: EnumError::SyntaxError
                }}
            },
            _ => {return AResult {
                result: EnumResult::Accept,
                action: vec![],
                guide: "".to_string(),
                error: EnumError::SyntaxError
            }}
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}
