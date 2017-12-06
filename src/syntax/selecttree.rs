use std::collections::HashMap;
use std::borrow::Borrow;
use std::iter::Iterator;
use super::super::analyse::dfa::{DfaWord};
use super::structures::{Switch, Syntax, EmptySyntax, SelectSyntax, Expression};
use super::trees::{DfaNode, AResult, EnumError, EnumResult, Tree};
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
//select语句语法树。
pub struct SelectTree {
    nodeset:HashMap<String, Box<DfaNode>>,
    error: (i32, EnumError)
}
impl SelectTree {
    pub fn new() -> Self {
        let mut map:HashMap<String, Box<DfaNode>> = HashMap::new();
        map.insert("Start".to_string(), Box::new(NodeStart{}));
        map.insert("Goal".to_string(), Box::new(NodeGoal{}));
        map.insert("Behind".to_string(), Box::new(NodeBehind{}));
        map.insert("From".to_string(), Box::new(NodeFrom{}));
        map.insert("StandardTable".to_string(), Box::new(NodeStandardTable{}));
        map.insert("Othername1".to_string(), Box::new(NodeOthername1{}));
        map.insert("Othername2".to_string(), Box::new(NodeOthername2{}));
        map.insert("SubSelect".to_string(), Box::new(NodeSubSelect{}));
        map.insert("OrderCheck".to_string(), Box::new(NodeOrderCheck{}));
        map.insert("Order".to_string(), Box::new(NodeOrder{}));
        map.insert("OrderColumn".to_string(), Box::new(NodeOrderColumn{}));
        map.insert("OrderTable".to_string(), Box::new(NodeOrderTable{}));
        map.insert("Where".to_string(), Box::new(NodeWhere{}));
        map.insert("Othername3".to_string(), Box::new(NodeOthername3{}));
        map.insert("Othername4".to_string(), Box::new(NodeOthername4{}));
        Self {
            nodeset: map,
            error: (0, EnumError::None)
        }
    }
}
impl SelectTree {
    pub fn construct(&mut self, li:&[DfaWord]) -> SelectSyntax {
        // for x in li {
        //     print!("{}, ", dfa_name(x));
        // }println!("");
        //select语法树会分析并返回SelectSyntax结构。
        let mut distinct = false;
        let mut froms:HashMap<String, Switch<String, SelectSyntax>> = HashMap::new();
        let mut goals:Vec<(String, Expression)> = Vec::new();
        let mut wheres = Expression::empty();// todo 需要重写。
        let mut orders:Vec<(String, bool)> = Vec::new();
        let mut last_from = "".to_string();
        let mut last_goal = "".to_string();
        let mut last_order = "".to_string();
        /* select语法树支持的action规则有：
            Goal:
                dist [value] 改变distinct的参数值。
                goal [begin] [end] 标记起点与终点，构造一个序列的表达式。
            From:
                from [name] 新的标准来源表名。
                fromsub [begin] [end] 标记始末，处理为一个序列表达式构成子查询。
                as [name] 将上一个加入的来源重命名。
        */
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
                        "dist" => {
                            distinct = res.action[1].parse().unwrap();
                        },
                        "goal" => {
                            let begin_i:usize = res.action[1].parse().unwrap();
                            let end_i:usize = res.action[2].parse().unwrap();
                            // 调用expression的构造。
                            let goal = Expression::new(&li[begin_i..end_i]);
                            let mut goal_name = String::new(); //合成默认的goalname
                            for i in &li[begin_i..end_i] {
                                goal_name += i.to_code_string().as_str();
                            }
                            goals.push((goal_name.to_string(),goal));
                            last_goal = goal_name.to_string();
                        },
                        "from" => {
                            let name = &res.action[1];
                            froms.insert(name.to_string(), Switch::One(name.to_string()));
                            last_from = name.to_string();
                        },
                        "fromsub" => {
                            let begin_i:usize = res.action[1].parse().unwrap();
                            let end_i:usize = res.action[2].parse().unwrap();
                            // 首先进行合法性检查，然后构造子查询。
                            let mut err = false;
                            if end_i - begin_i < 3 {err = true;}
                            else {
                                if let DfaWord::Kword(ref word) = li[begin_i+1] {
                                    let mut sub = SelectTree::new();
                                    froms.insert("SubSelect".to_string(), Switch::Two(sub.construct(&li[begin_i+2..end_i])));
                                    last_from = "SubSelect".to_string();
                                }else {err = true;}
                            }
                            if err {
                                self.error = (begin_i as i32, EnumError::SyntaxError);
                                break;
                            }
                        },
                        "as" => {
                            let new_name:String = res.action[1].to_string();
                            if last_from != "" {
                                if let Option::Some(t) = froms.remove(&last_from) {
                                    froms.insert(new_name, t);
                                }
                            }else if last_goal != "" {
                                let mut goal_i:usize = 0;
                                for i in &goals {
                                    if i.0 == last_goal {
                                        break;
                                    }
                                    goal_i += 1;
                                }
                                if goal_i < goals.len() {
                                    let mut t = goals.remove(goal_i);
                                    t.0 = new_name;
                                    goals.insert(goal_i, t);
                                }
                            }
                            last_from = "".to_string();
                            last_goal = "".to_string();
                        },
                        "where" => {
                            let begin_i:usize = res.action[1].parse().unwrap();
                            let end_i:usize = res.action[2].parse().unwrap();
                            // 调用expression的构造。
                            wheres = Expression::new(&li[begin_i..end_i]);
                        },
                        "order" => {
                            let name = &res.action[1];
                            orders.push((name.to_string(), true));
                            last_order = name.to_string();
                        },
                        "orderdesc" => {
                            let mut index = 0;
                            while index < orders.len() {
                                if orders[index].0 == last_order.as_str() {
                                    break;
                                }
                            }
                            if index < orders.len() {
                                let (name, _) = orders.remove(index);
                                orders.insert(index, (name.to_string(), false));
                                last_order = "".to_string();
                            }
                        },
                        "ordertable" => {
                            let mut index = 0;
                            while index < orders.len() {
                                if orders[index].0 == last_order.as_str() {
                                    break;
                                }
                            }
                            if index < orders.len() {
                                let v = &res.action[1];
                                let (name, desc) = orders.remove(index);
                                let new_name = format!("{}.{}", name, v);
                                orders.insert(index, (new_name.to_string(), desc));
                                last_order = new_name;
                            }
                        }
                        _ => {

                        }
                    }
                }
                if res.guide != "" {
                    node = self.nodeset[&res.guide].borrow();
                }
            }
        }
        SelectSyntax{
            distinct: distinct,
            froms: froms,
            goals: goals,
            wheres: wheres,
            orders: orders
        }
    }
    pub fn get_error(&self) -> &(i32, EnumError){
        &self.error
    }
}

//= 节点 ============================================
struct NodeStart;
impl DfaNode for NodeStart {
    fn analysis(&self, w:&DfaWord) -> AResult {
        match w {
            &DfaWord::Var(_)|
            &DfaWord::Integer(_)|
            &DfaWord::Float(_)|
            &DfaWord::Str(_)|
            &DfaWord::Bool(_) => AResult{
                result: EnumResult::Return,
                action: vec![],
                guide: "Goal".to_string(),
                error: EnumError::None
            },
            &DfaWord::Signal(ref s) if s != "," => AResult{
                result: EnumResult::Return,
                action: vec![],
                guide: "Goal".to_string(),
                error: EnumError::None
            },
            &DfaWord::Kword(ref word) if word == "all" || word == "distinct" => AResult{
                result: EnumResult::Accept,
                action: vec!["dist".to_string(), if word == "distinct"{"true"}else{"false"}.to_string()],
                guide: "".to_string(),
                error: EnumError::None
            },
            _ => AResult {
                result:EnumResult::Accept,
                action: vec![],
                guide: "".to_string(),
                error: EnumError::SyntaxError
            }
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeGoal;
impl DfaNode for NodeGoal {
    fn analysis(&self, w:&DfaWord) -> AResult {
        panic!("Not Allowed");
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        // 构造目标列.方案是在这里通过括号配对找到终点，然后输出终点位置。
        let mut i = begin as usize;
        while i < w.len() {
            if let DfaWord::Kword(ref k) = w[i] {
                *end = (if k == "as" {i+1}else{i}) as i32;
                return AResult {
                    result: if k == "as" {EnumResult::Accept}else{EnumResult::Return},
                    guide: if k == "as" {"Othername3".to_string()}else{"Behind".to_string()},
                    error: EnumError::None,
                    action: vec!["goal".to_string(), begin.to_string(), i.to_string()]
                }
            }else if let DfaWord::Signal(ref s) = w[i] {
                if s == "," {
                    *end = (i+1) as i32;
                    return AResult {
                        result: EnumResult::Accept,
                        guide: "".to_string(),
                        error: EnumError::None,
                        action: vec!["goal".to_string(), begin.to_string(), i.to_string()]
                    }
                }else {
                    i+=1;
                }
            }else{
                i+=1;
            }
        }
        *end = w.len() as i32;
        AResult {
            result: EnumResult::Accept,
            guide: "Behind".to_string(),
            action: vec!["goal".to_string(), begin.to_string(), w.len().to_string()],
            error: EnumError::None
        }
    }
    fn allow_array(&self) -> bool {true}
}

struct NodeBehind;
impl DfaNode for NodeBehind {
    fn analysis(&self, w:&DfaWord) -> AResult {
        if let &DfaWord::Kword(ref word) = w {
            let mut guide = "";
            match &word[..] {
                "from" => {guide = "From";},
                "where" => {guide = "Where";},
                "group" => {guide = "GroupCheck";},
                "order" => {guide = "OrderCheck";},
                _ => {return AResult{
                    result: EnumResult::Accept,
                    action: vec![],
                    guide: "".to_string(),
                    error: EnumError::SyntaxError
                }}
            }
            return AResult {
                result: EnumResult::Accept,
                action: vec![],
                guide: guide.to_string(),
                error: EnumError::None
            }
        }else{
            return AResult {
                result: EnumResult::Accept,
                action: vec![],
                guide: "".to_string(),
                error: EnumError::SyntaxError
            }
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
            &DfaWord::Kword(ref word) => {
                result = EnumResult::Return;
                guide = "Behind";
            },
            &DfaWord::Var(ref v) => {
                action = vec!["from".to_string(), v.to_string()];
                guide = "StandardTable";
            },
            &DfaWord::Signal(ref s) if s == "(" => {
                result = EnumResult::Return;
                guide = "SubSelect";
            },
            _ => {
                error = EnumError::SyntaxError;
            }
        }
        AResult {
            result: result,
            action: action,
            guide: guide.to_string(),
            error: error
        }
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        panic!("Not Allowed");
    }
    fn allow_array(&self) -> bool {false}
}

struct NodeStandardTable;
impl DfaNode for NodeStandardTable {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(ref word) if word == "as" => {
                guide = "Othername1";
            },
            &DfaWord::Kword(_) => {
                result = EnumResult::Return;
                guide = "From";
            },
            &DfaWord::Signal(ref s) if s == "," => {
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

struct NodeOthername1;
impl DfaNode for NodeOthername1 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        if let &DfaWord::Var(ref v) = w {
            action = vec!["as".to_string(), v.to_string()];
            guide = "Othername2";
        }else {
            error = EnumError::SyntaxError;
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

struct NodeOthername2;
impl DfaNode for NodeOthername2 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Kword(_) => {
                guide = "From";
                result = EnumResult::Return;
            },
            &DfaWord::Signal(ref s) if s == "," => {
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

struct NodeOthername3;
impl DfaNode for NodeOthername3 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Var(ref k) => {
                guide = "Othername4";
                action = vec!["as".to_string(), k.to_string()];
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

struct NodeOthername4;
impl DfaNode for NodeOthername4 {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        match w {
            &DfaWord::Signal(ref k) if k == "," => {
                guide = "Goal";
            },
            &DfaWord::Var(..) | &DfaWord::Integer(..) | &DfaWord::Float(..) | &DfaWord::Bool(..) |
            &DfaWord::Str(..) | &DfaWord::Signal(..) => {
                guide = "Goal";
                result = EnumResult::Return;
            },
            &DfaWord::Kword(..) => {
                guide = "Behind";
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

struct NodeSubSelect;
impl DfaNode for NodeSubSelect {
    fn analysis(&self, w:&DfaWord) -> AResult {
        panic!("Not Allowed");
    }
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        //需要用括号配对找到终括号。
        let mut stack_k = 0;
        let mut i = begin as usize;
        while i < w.len() {
            match w[i] {
                DfaWord::Signal(ref s) if s == "(" => {
                    stack_k += 1;
                },
                DfaWord::Signal(ref s) if s == ")" => {
                    stack_k -=1;
                },
                _ => {}
            }
            if stack_k <= 0 {
                break;
            }else{
                i+=1;
            }
        }
        action = vec!["fromsub".to_string(), begin.to_string(), i.to_string()];
        //上述代码确定终点i的位置。
        //然后判断下一个的内容。
        if i + 1 < w.len() {
            match w[i + 1] {
                DfaWord::Kword(ref word) if word == "as" => {
                    guide = "Othername1";
                    *end = (i+2) as i32;
                },
                DfaWord::Kword(_) => {
                    guide = "From";
                    //result = EnumResult::Return;
                    *end = (i + 1) as i32;
                },
                DfaWord::Signal(ref s) if s == "," => {
                    guide = "From";
                    *end = (i+2) as i32;
                },
                _ => {
                    error = EnumError::SyntaxError;
                    *end = (i+2) as i32;
                }
            }
        }else {
            guide = "From";
            *end = (i+1) as i32;
        }
        return AResult {
            result: result, action: action, guide: guide.to_string(), error: error
        }
    }
    fn allow_array(&self) -> bool {true}
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
                    guide: "Behind".to_string(),
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
            guide: "Behind".to_string(),
            action: vec!["where".to_string(), begin.to_string(), w.len().to_string()],
            error: EnumError::None
        }
    }
    fn allow_array(&self) -> bool {true}
}

struct NodeOrderCheck;
impl DfaNode for NodeOrderCheck {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        if let &DfaWord::Kword(ref word) = w {
            if word == "by" {
                guide = "Order";
            }else{
                guide = "Behind";
                result = EnumResult::Return;
            }
        }else{
            error = EnumError::SyntaxError;
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

struct NodeOrder;
impl DfaNode for NodeOrder {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        if let &DfaWord::Var(ref var) = w {
            guide = "OrderColumn";
            action = vec!["order".to_string(), var.to_string()];
        }else if let &DfaWord::Kword(_) = w {
            guide = "OrderCheck";
            result = EnumResult::Return;
        }else{
            error = EnumError::SyntaxError;
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

struct NodeOrderColumn;
impl DfaNode for NodeOrderColumn {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        if let &DfaWord::Kword(ref kword) = w {
            if kword == "desc" {
                action = vec!["orderdesc".to_string()];
            }else{
                result = EnumResult::Return;
                guide = "Order";
            }
        }else if let &DfaWord::Signal(ref signal) = w {
            if signal == "," {
                guide = "Order";
            }else if signal == "." {
                guide = "OrderTable";
            }else{
                error = EnumError::IllegalSignal;
            }
        }else {
            error = EnumError::SyntaxError;
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

struct NodeOrderTable;
impl DfaNode for NodeOrderTable {
    fn analysis(&self, w:&DfaWord) -> AResult {
        let mut result = EnumResult::Accept;
        let mut action = vec![];
        let mut guide = "";
        let mut error = EnumError::None;
        if let &DfaWord::Var(ref var) = w {
            action = vec!["ordertable".to_string(), var.to_string()];
            guide = "OrderColumn";
        }else{
            error = EnumError::SyntaxError;
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


// {
//     fn analysis(&self, w:&DfaWord) -> AResult {
//         let mut result = EnumResult::Accept;
//         let mut action = vec![];
//         let mut guide = "";
//         let mut error = EnumError::None;

//         return AResult {
//             result: result, action: action, guide: guide.to_string(), error: error
//         }
//     }
//     fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult{
//         panic!("Not Allowed");
//     }
//     fn allow_array(&self) -> bool {false}
// }