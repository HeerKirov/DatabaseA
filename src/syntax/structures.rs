use std::collections::HashMap;
use std::convert::From;
use std::clone::Clone;
use super::super::analyse::dfa::{DfaWord};

pub enum Switch<A, B> {
    One(A),
    Two(B)
}
//表达式结构体。
pub enum ExpressionType {
    Kword(String),
    Var(Vec<String>),
    Bool(bool),
    Integer(i64),
    Float(f64),
    Str(String),
    Signal(String)
}
impl ExpressionType {
    pub fn new(d:&DfaWord) -> Self {
        match d {
            &DfaWord::Kword(ref word) => ExpressionType::Kword(word.to_string()),
            &DfaWord::Integer(ref i) => ExpressionType::Integer(*i),
            &DfaWord::Float(ref f) => ExpressionType::Float(*f),
            &DfaWord::Str(ref s) => ExpressionType::Str(s.to_string()),
            &DfaWord::Signal(ref s) => ExpressionType::Signal(s.to_string()),
            &DfaWord::Var(ref v) => {
                if v == "true" {ExpressionType::Bool(true)}
                else if v == "false" {ExpressionType::Bool(false)}
                else {ExpressionType::Var(vec![v.to_string()])}
            },
            _ => {panic!("ALERT:NO THIS TYPE.");}
        }
    }
    pub fn new_var(e:&ExpressionType, d:&ExpressionType) -> Self {
        if let &ExpressionType::Var(ref v) = e {
            if let &ExpressionType::Var(ref vv) = d {
                let mut v2 = vec![];
                for i in v.iter() {
                    v2.push(i.to_string());
                }
                for i in vv.iter(){
                    v2.push(i.to_string());
                }
                return ExpressionType::Var(v2);
            }
        }
        panic!("ALERT:TYPE ERROR.");
    }
    pub fn to_string(&self) -> String {
        match self {
            &ExpressionType::Kword(ref w) => w.to_string(),
            &ExpressionType::Var(ref v) => {
                v.join(".").to_string()
            },
            &ExpressionType::Integer(ref i) => i.to_string(),
            &ExpressionType::Float(ref f) => f.to_string(),
            &ExpressionType::Str(ref s) => s.to_string(),
            &ExpressionType::Signal(ref s) => s.to_string(),
            &ExpressionType::Bool(ref b) => b.to_string()
        }
    }
    pub fn copy(&self) -> Self {
        match self {
            &ExpressionType::Kword(ref w) => ExpressionType::Kword(w.to_string()),
            &ExpressionType::Var(ref v) => {
                let mut nw = vec![];
                for i in v.iter(){nw.push(i.to_string());}
                ExpressionType::Var(nw)
            },
            &ExpressionType::Integer(i) => ExpressionType::Integer(i),
            &ExpressionType::Float(f) => ExpressionType::Float(f),
            &ExpressionType::Str(ref s) => ExpressionType::Str(s.to_string()),
            &ExpressionType::Signal(ref s) => ExpressionType::Signal(s.to_string()),
            &ExpressionType::Bool(b) => ExpressionType::Bool(b)
        }
    }
    pub fn abs_eq(a:f64, b:f64) -> bool {
        (a-b).abs() < 1e-10
    }
    pub fn make_two(p1:&ExpressionType, p2:&ExpressionType, oper:&str) -> Result<ExpressionType, String> {
        match oper {
            "^" => {
                match p1 {
                    &ExpressionType::Integer(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Float((a as f64).powi(b as i32))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float((a as f64).powf(b))),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Float(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Float(a.powi(b as i32))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float(a.powf(b))),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            "*" => {
                match p1 {
                    &ExpressionType::Integer(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Integer(a*b)),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float((a as f64)*b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Float(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Float(a*(b as f64))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float(a*b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            "/" => {
                match p1 {
                    &ExpressionType::Integer(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Integer(a/b)),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float((a as f64)/b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Float(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Float(a/(b as f64))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float(a/b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            "%" => {
                match p1 {
                    &ExpressionType::Integer(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Integer(a%b)),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float((a as f64)%b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Float(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Float(a%(b as f64))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float(a%b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            "+" => {
                match p1 {
                    &ExpressionType::Integer(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Integer(a+b)),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float((a as f64)+b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Float(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Float(a+(b as f64))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float(a+b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Str(ref a) => {
                        match p2 {
                            &ExpressionType::Str(ref b) => Result::Ok(ExpressionType::Str(format!("{}{}", a, b))),
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Str(format!("{}{}", a, b))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Str(format!("{}{}", a, b))),
                            &ExpressionType::Bool(b) => Result::Ok(ExpressionType::Str(format!("{}{}", a, b))),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    }
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            "-" => {
                match p1 {
                    &ExpressionType::Integer(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Integer(a-b)),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float((a as f64)-b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Float(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Float(a-(b as f64))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Float(a-b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            ">=" => {
                match p1 {
                    &ExpressionType::Integer(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Bool(a>=b)),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Bool((a as f64)>=b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Float(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Bool(a>=(b as f64))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Bool(a>=b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            "<=" => {
                match p1 {
                    &ExpressionType::Integer(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Bool(a<=b)),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Bool((a as f64)<=b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Float(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Bool(a<=(b as f64))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Bool(a<=b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            ">" => {
                match p1 {
                    &ExpressionType::Integer(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Bool(a>b)),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Bool((a as f64)>b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Float(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Bool(a>(b as f64))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Bool(a>b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            "<" => {
                match p1 {
                    &ExpressionType::Integer(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Bool(a<b)),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Bool((a as f64)<b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Float(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Bool(a<(b as f64))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Bool(a<b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            "=" => {
                match p1 {
                    &ExpressionType::Integer(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Bool(a==b)),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Bool(ExpressionType::abs_eq(a as f64, b))),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Float(a) => {
                        match p2 {
                            &ExpressionType::Integer(b) => Result::Ok(ExpressionType::Bool(ExpressionType::abs_eq(a, b as f64))),
                            &ExpressionType::Float(b) => Result::Ok(ExpressionType::Bool(ExpressionType::abs_eq(a, b))),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Str(ref a) => {
                        match p2 {
                            &ExpressionType::Str(ref b) => Result::Ok(ExpressionType::Bool(a==b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    &ExpressionType::Bool(a) => {
                        match p2 {
                            &ExpressionType::Bool(b) => Result::Ok(ExpressionType::Bool(a==b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    }
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            "!=" => match ExpressionType::make_two(p1, p2, "=") {
                Result::Ok(ok) => {
                    if let ExpressionType::Bool(b) = ok {Result::Ok(ExpressionType::Bool(!b))}
                    else{Result::Err(format!("Wrong operator param type."))}
                },
                e@Result::Err(_) => e
            },
            "&&" => {
                match p1 {
                    &ExpressionType::Bool(a) => {
                        match p2 {
                            &ExpressionType::Bool(b) => Result::Ok(ExpressionType::Bool(a&&b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            },
            "||" => {
                match p1 {
                    &ExpressionType::Bool(a) => {
                        match p2 {
                            &ExpressionType::Bool(b) => Result::Ok(ExpressionType::Bool(a||b)),
                            _ => Result::Err(format!("Wrong operator param type."))
                        }
                    },
                    _ => Result::Err(format!("Wrong operator param type."))
                }
            }
            _ => Result::Err(format!("Unknown operator: {}.", oper))
        }
    }
    pub fn make_one(p1:&ExpressionType, oper:&str) -> Result<ExpressionType, String> {
        match oper {
            "!" => match p1 {
                &ExpressionType::Bool(a) => Result::Ok(ExpressionType::Bool(!a)),
                 _ => Result::Err(format!("Wrong operator param type."))
            },
             _ => Result::Err(format!("Wrong operator param type."))
        }
    }
}
pub struct Expression {
    pub li:Vec<ExpressionType>,
    pub setence: String
}
impl Expression {
    fn lv(s:&str) -> i32 {
        let mut map:HashMap<i32, Vec<&str>> = HashMap::new();
        map.insert(0, ["(", ")"].to_vec());
        map.insert(999, ["."].to_vec());
        map.insert(10, ["^"].to_vec());
        map.insert(9, ["*", "/", "%"].to_vec());
        map.insert(8,["+", "-"].to_vec());
        map.insert(7, [">", "<", ">=" ,"<=", "=", "!="].to_vec());
        map.insert(6, ["&&", "||"].to_vec());
        map.insert(4, ["!"].to_vec());
        for (i, v) in map {
            for j in v {
                if j == s {
                    return i;
                }
            }
        }
        return -1;
    }
    pub fn empty() -> Self {
        Expression::new(&vec![])
    }
    pub fn new_single(s:&str) -> Self {
        Expression::new(&vec![DfaWord::Var(s.to_string())])
    }
    pub fn new_allin() -> Self {
        Expression::new(&vec![DfaWord::Signal("*".to_string())])
    }
    pub fn new(li:&[DfaWord]) -> Self {
        //构造一个没有逗号和分号分割的表达式。传入表达式的中缀模式。
        //使用中转后模式。
        //特殊处理点语法。
        let mut ret:Vec<ExpressionType> = Vec::new();
        let mut stack:Vec<ExpressionType> = Vec::new();
        //开始。
        for i in li {
            //首先区分符号与计算对象。
            match i {
                &DfaWord::Signal(ref s) => {
                    //遇到一个符号时，需要与栈顶作比较。当新符号Lv<=栈顶Lv时，需要出栈。
                    if s == "(" {
                        stack.push(ExpressionType::new(i));
                    }else if s == ")" {
                        //右括号直接收栈。
                        loop {
                            if let Option::Some(t) = stack.pop() {
                                if let ExpressionType::Signal(ref s) = t {
                                    if s == "(" {
                                        break;
                                    }
                                }
                                ret.push(t);
                            }else{
                                break;
                            }
                        }
                    }else{
                        let current = Expression::lv(s);
                        loop {
                            let mut out = false;
                            match stack.last() {
                                None => {break;}
                                Some(ref t) => {
                                    if let &ExpressionType::Signal(ref sign) = *t {
                                        if current <= Expression::lv(sign) {
                                            out = true;
                                        } else {break;}
                                    }
                                } 
                            }
                            if out {
                                ret.push(stack.pop().unwrap());
                            }
                        }
                        stack.push(ExpressionType::new(i));    
                    }  
                },
                &DfaWord::End | &DfaWord::Kword(_) => {},
                _ => {//var,int,float,str
                    ret.push(ExpressionType::new(&i));
                }
            }
            // print!("RET: ");
            // for i in ret.iter() {print!("[{}]", i.to_string());}
            // print!("\nSTACK: ");
            // for i in stack.iter() {print!("[{}]", i.to_string());}
            // println!("");
        }
        while !stack.is_empty() {
            match stack.pop() {
                None => {break;}
                Some(t) => {ret.push(t);}
            }
        }
        //最后，在返回之前，需要立即计算点符号，生成var。
        let mut i = 0;
        let mut len = ret.len();
        while i < len {
            let mut flag = false;
            if let ExpressionType::Signal(ref s) = ret[i] {
                if s == "." {flag = true;}
            }
            if flag && i >= 2 {
                ret.remove(i);
                //要求i位置之前至少有两个元素，且这两个元素都是var类型。
                if let v2@ExpressionType::Var(..) = ret.remove(i-1) {
                    if let v1@ExpressionType::Var(..) = ret.remove(i-2) {
                        ret.insert(i-2, ExpressionType::new_var(&v1, &v2));
                        i -= 2;
                        len -= 2;
                    }
                }
            }
            i += 1;
        }
        let mut set = String::new();
        for i in li.iter() {
            set+=i.to_code_string().as_str();
        }
        Self{
            li:ret,
            setence: set
        }
    }
    pub fn new_array(li:&[DfaWord]) -> Vec<Self> {
        let mut v:Vec<Self> = Vec::new();
        let mut i = 0;
        let mut j = 0;
        while j < li.len() {
            match li[j] {
                DfaWord::Signal(ref s) if s == "," => {
                    if j > i {
                        v.push(Expression::new(&li[i..j-1]));
                        i = j + 1;
                    }
                },
                _ => {}
            }
            j += 1;
        }
        v
    }
    pub fn to_string(&self) -> String {
        let mut ret = "".to_string();
        // print!("len={}", self.li.len());
        for i in &self.li {
         ret += &(i.to_string() + " ");   
        }
        ret
    }
    pub fn copy(&self) -> Self {
        let mut li = vec![];
        for i in self.li.iter() {
            li.push(i.copy());
        }
        Self{li:li, setence: self.setence.to_string()}
    }
}
//= 组合结构体 =============================================
pub enum ColSyntax {
    None,
    Select(SelectSyntax),
    Insert(InsertSyntax),
    Update(UpdateSyntax),
    Delete(DeleteSyntax),
    CreateTable(CreateTableSyntax),
    AlterTable(AlterTableSyntax),
    DropTable(DropTableSyntax),
    CreateDatabase(CreateDatabaseSyntax),
    DropDatabase(DropDatabaseSyntax),
    Use(UseSyntax),
    Help(HelpSyntax),
    CreateUser(CreateUserSyntax),
    AlterUser(AlterUserSyntax),
    DropUser(DropUserSyntax),
    Grant(GrantSyntax),
    CreateView(CreateViewSyntax),
    DropView(DropViewSyntax)
}

//= 接口 ==========================================================================
//顶级语法树的接口。
pub trait Syntax {
    fn get_type(&self) -> String;
    fn to_string(&self) -> String {
        "".to_string()
    }
}
pub struct EmptySyntax;
impl Syntax for EmptySyntax {
    fn get_type(&self) -> String {"empty".to_string()}
}

//= Select语法树 ==================================================================
pub struct SelectSyntax {
    pub distinct:bool,
    pub froms:HashMap<String, Switch<String, SelectSyntax>>, //来源的别名:实际列
    pub goals: Vec<(String, Expression)>, //目标列表，包括别名。
    pub wheres:Expression, //条件表达式
    pub orders:Vec<(String, bool)> //排序序列
}
impl SelectSyntax {
    pub fn copy(&self) -> Self {
        let mut froms = HashMap::new();
        for (ref k, v) in self.froms.iter() {
            froms.insert(k.to_string(), match v{
                &Switch::One(ref s) => Switch::One(s.to_string()),
                &Switch::Two(ref s) => Switch::Two(s.copy())
            });
        }
        let mut goals = Vec::new();
        for &(ref k, ref v) in self.goals.iter() {
            goals.push((k.to_string(), v.copy()));
        }
        let mut orders = Vec::new();
        for &(ref k, v) in self.orders.iter() {
            orders.push((k.to_string(), v));
        }
        Self {
            distinct: self.distinct,
            wheres: self.wheres.copy(),
            froms: froms,
            goals: goals,
            orders: orders
        }
    }
    pub fn get_setence(&self) -> String {
        let mut ret = format!("SELECT ");

        let len = self.goals.len();
        for (i, &(ref name, ref exp)) in self.goals.iter().enumerate() {
            ret += format!("{} AS {}", exp.setence, name).as_str();
            if i < len - 1 {ret += ", ";}else{ret += " ";}
        }
        if !self.froms.is_empty() {
            ret += "\nFROM ";
            let len = self.froms.len();
            for (i, (ref name, switch)) in self.froms.iter().enumerate() {
                match switch {
                    &Switch::One(ref s) => {ret += format!("{}", s).as_str();},
                    &Switch::Two(ref sub) => {ret += format!("({})", sub.get_setence()).as_str();}
                }
                if i < len - 1 {ret += ", "}else{ret += " ";}
            }
        }
        if self.wheres.li.len() > 0 {
            ret += format!("\nWHERE {}", self.wheres.setence).as_str();
        }
        if !self.orders.is_empty() {
            ret += "\nORDER BY ";
            let len = self.froms.len();
            for (i, &(ref name, b)) in self.orders.iter().enumerate() {
                ret += format!("{}{}{} ", if b {""}else{"-"}, name, if i < len - 1 {","}else{""}).as_str();
            }
        }
        ret
    }
}
impl Syntax for SelectSyntax {
    fn get_type(&self) -> String {"select".to_string()}
    fn to_string(&self) -> String {
        let dist = self.distinct.to_string();
        let mut from = "".to_string();
        for (a, s) in &self.froms {
            from += &format!("    {}: {}\n", a, &match s {
                &Switch::One(ref a) => a.to_string(),
                &Switch::Two(ref b) => b.to_string()
            });
        }
        let mut goal = "".to_string();
        for t in &self.goals {
            goal += &format!("    {}: {}\n", t.0, t.1.to_string());
        }
        format!("dist:{}\nfroms:\n{}goals:\n{}", dist, from, goal)
    }
}
//= Insert语法树 ===========================================
pub struct InsertSyntax {
    pub table_name:String,
    pub has_head:bool,
    pub values: Vec<HashMap<String, DfaWord>>
}
impl Syntax for InsertSyntax {
    fn get_type(&self) -> String {"insert".to_string()}
}
//= Delete语法树 ================================
pub struct DeleteSyntax {
    pub table_name:String,
    pub wheres: Expression
}
impl Syntax for DeleteSyntax {
    fn get_type(&self) -> String {"delete".to_string()}
}
//= Update语法树 =============================
pub struct UpdateSyntax {
    pub table_name: String,
    pub wheres: Expression,
    pub sets: HashMap<String, DfaWord>
}
impl Syntax for UpdateSyntax {
    fn get_type(&self) -> String {"update".to_string()}
}
//= CreateTable语法树 =========================
pub struct CreateTableSyntax {
    pub name: String,
    pub fields: Vec<TableFieldSyntax>,
    pub foreigns: Vec<TableForeignSyntax>
}
pub struct TableFieldSyntax {
    pub name:String,
    pub t: String,
    pub unique: bool,
    pub primary: bool,
    pub not_null: bool,
    pub default: Option<String>,
    pub auto_inc: bool
}
impl TableFieldSyntax {
    pub fn empty() -> Self {Self{
        name: "".to_string(), t: "".to_string(), unique: false, primary: false, not_null: false, default: Option::None, auto_inc: false
    }}
}
pub struct TableForeignSyntax {
    pub field: String,  // 在本表中的键名
    pub foreign_table: String,  // 外表表名
    pub foreign_field: String,  // 在外表中的field
    pub delete_action: String  // 删除操作类型
}
impl TableForeignSyntax {
    pub fn empty() -> Self{Self{
        field: "".to_string(), foreign_table: "".to_string(), foreign_field: "".to_string(), delete_action: "".to_string()
    }}
}
impl Syntax for CreateTableSyntax {
    fn get_type(&self) -> String {"create_table".to_string()}
}

//= AlterTable语法树 ============================
pub struct AlterTableSyntax {
    pub name: String,
    pub adds: Vec<TableFieldSyntax>,
    pub alters: Vec<TableFieldSyntax>,
    pub drops: Vec<String>
}
impl Syntax for AlterTableSyntax {
    fn get_type(&self) -> String {"alter_table".to_string()}
}
//= DropTable语法树 ========================
pub struct DropTableSyntax {
    pub name: String
}
impl DropTableSyntax {
    pub fn new(name:&str) -> Self {
        Self {name: name.to_string()}
    }
}
impl Syntax for DropTableSyntax {
    fn get_type(&self) -> String {"drop_table".to_string()}
}
//= CreateDatabase语法树 ===============
pub struct CreateDatabaseSyntax {
    pub name:String
}
impl CreateDatabaseSyntax {
    pub fn new(name:&str) -> Self {
        Self {name: name.to_string()}
    }
}
impl Syntax for CreateDatabaseSyntax {
    fn get_type(&self) -> String {"create_database".to_string()}
}
//= DropDatabase语法树====================
pub struct DropDatabaseSyntax {
    pub name:String
}
impl DropDatabaseSyntax {
    pub fn new(name:&str) -> Self {
        Self {name: name.to_string()}
    }
}
impl Syntax for DropDatabaseSyntax {
    fn get_type(&self) -> String {"drop_database".to_string()}
}
//= use语法树 =====================
pub struct UseSyntax {
    pub name: String
}
impl UseSyntax {
    pub fn new(name:&str) -> Self {
        Self {name: name.to_string()}
    }
}
impl Syntax for UseSyntax {
    fn get_type(&self) -> String {"use".to_string()}
}
//= help语法树 ====================
pub struct HelpSyntax {
    pub params: Vec<String>
}
impl Syntax for HelpSyntax {
    fn get_type(&self) -> String {"help".to_string()}
}
//= create user 语法树 ===========
pub struct CreateUserSyntax {
    pub username: String,
    pub password: String,
    pub staff: bool
}
impl Syntax for CreateUserSyntax {
    fn get_type(&self) -> String {"create_user".to_string()}
}
impl CreateUserSyntax {
    pub fn new(user:&str, pw:&str, staff: bool) -> Self{Self{
        username: user.to_string(),
        password: pw.to_string(),
        staff: staff
    }}
}
//= alter user 语法树 ============
pub struct AlterUserSyntax {
    pub username: String,
    pub password: String
}
impl Syntax for AlterUserSyntax {
    fn get_type(&self) -> String {"alter_user".to_string()}
}
impl AlterUserSyntax {
    pub fn new(user:&str, pw:&str) -> Self{Self{
        username: user.to_string(),
        password: pw.to_string()
    }}
}
//= drop user 语法树 =============
pub struct DropUserSyntax {
    pub username:String
}
impl Syntax for DropUserSyntax {
    fn get_type(&self) -> String {"drop_user".to_string()}
}
impl DropUserSyntax {
    pub fn new(user:&str) -> Self{Self{
        username: user.to_string()
    }}
}
//= grant 语法树 =================
pub struct GrantSyntax {
    pub grants: Vec<String>,
    pub all: bool,
    pub objects:Vec<(String, String)>,
    pub users: Vec<String>,
    pub is_grant: bool
}
impl Syntax for GrantSyntax {
    fn get_type(&self) -> String {"grant".to_string()}
}
//= create view 语法树 ============
pub struct CreateViewSyntax {
    pub name: String,
    pub sub: SelectSyntax
}
impl CreateViewSyntax {
    pub fn new(name:&str, syntax:SelectSyntax) -> Self {Self{
        name: name.to_string(),
        sub: syntax
    }}
}
impl Syntax for CreateViewSyntax {
    fn get_type(&self) -> String {"createview".to_string()}
}
//= drop view 语法树 ===============
pub struct DropViewSyntax {
    pub name: String
}
impl DropViewSyntax {
    pub fn new(name:&str) -> Self{Self{
        name: name.to_string()
    }}
}
impl Syntax for DropViewSyntax {
    fn get_type(&self) -> String {"dropview".to_string()}
}