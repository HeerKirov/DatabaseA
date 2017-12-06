use std::collections::HashMap;
use std::marker::Sized;
use std::borrow::Borrow;
use super::utils::CharUtils;

#[derive(Hash, Eq, Copy, Clone)]
enum EnumResult {
    Accept = 0, //接受
    AcceptAndNo = 1, //接受但不放入队列
    Clear = 2, //抛弃当前队列
    Return = 3, //放回
}
impl PartialEq for EnumResult {
    fn eq(&self, other: &EnumResult) -> bool {
        *self as i32 == *other as i32
    }
}

#[derive(Hash, Eq, Copy, Clone)]
enum EnumOutput {
    None = 0, //没有结果
    Kword = 1, Integer = 2, Float = 3, Str = 4, Signal = 5
}
impl PartialEq for EnumOutput {
    fn eq(&self, other: &EnumOutput) -> bool {
        *self as i32 == *other as i32
    }
}

#[derive(Hash, Eq, Copy, Clone)]
enum EnumGuide {
    Me = 0,
    Begin = 1, Kword = 2, Integer = 3, Float = 4,
    Str = 5, TransStr = 6, Signal = 7, 
    MultiS1 = 8, MultiS2 = 9, MultiS3 = 10, Comment1 = 11, Comment2 = 12, CheckComment = 13
}
impl PartialEq for EnumGuide {
    fn eq(&self, other: &EnumGuide) -> bool {
        *self as i32 == *other as i32
    }
}

#[derive(Hash, Eq, Copy, Clone)]
pub enum EnumError {
    None,
    UnknownSignal,
    IllegalDigit,
    UnexpectedEnd,
    ExternalError
}
impl PartialEq for EnumError {
    fn eq(&self, other: &EnumError) -> bool {
        *self as i32 == *other as i32
    }
}

struct AResult{
    sign:char, result:EnumResult, output:EnumOutput, guide: EnumGuide, error:EnumError
}

pub enum DfaWord{
    End,
    Kword(String),
    Var(String),
    Integer(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Signal(String)
}
impl DfaWord {
    pub fn to_code_string(&self) -> String {
        //输出code格式的string，主要用于构造表头时合成表达式。
        match self {
            &DfaWord::Kword(ref k) => k.to_uppercase(),
            &DfaWord::Var(ref v) => v.to_string(),
            &DfaWord::Integer(i) => i.to_string(),
            &DfaWord::Float(f) => f.to_string(),
            &DfaWord::Str(ref s) => format!("\"{}\"", s),
            &DfaWord::Bool(b) => b.to_string(),
            &DfaWord::Signal(ref s) => s.to_string(),
            &DfaWord::End => "".to_string()
        }
    }
    pub fn copy(&self) -> Self {
        match self {
            &DfaWord::End => DfaWord::End,
            &DfaWord::Kword(ref s) => DfaWord::Kword(s.to_string()),
            &DfaWord::Var(ref s) => DfaWord::Var(s.to_string()),
            &DfaWord::Integer(i) => DfaWord::Integer(i),
            &DfaWord::Float(f) => DfaWord::Float(f),
            &DfaWord::Str(ref s) => DfaWord::Str(s.to_string()),
            &DfaWord::Bool(b) => DfaWord::Bool(b),
            &DfaWord::Signal(ref s ) => DfaWord::Signal(s.to_string())
        }
    }
}

trait DfaNode {
    fn analysis(&self, c:char) -> AResult;
}

struct DfaBegin;
impl DfaNode for DfaBegin{
    fn analysis(&self, c:char) -> AResult {
        let mut ans = AResult{ 
            sign: c,
            result: EnumResult::Accept, 
            output: EnumOutput::None, 
            guide: EnumGuide::Me, 
            error: EnumError::None 
        };
        if c.is_signal(&['_']) {ans.guide = EnumGuide::Kword;}
        else if c.is_alphas() {ans.guide = EnumGuide::Kword;}
        else if c.is_digits() {ans.guide = EnumGuide::Integer;}
        else if c.is_std_signal() {
            ans.guide = EnumGuide::Signal;
            ans.result = EnumResult::Return;
        }else if c.is_space()||c.is_enter()||c.is_end() {ans.result = EnumResult::AcceptAndNo;}
        else {ans.error = EnumError::UnknownSignal;}
        ans
    }
}
struct DfaKword;
impl DfaNode for DfaKword{
    fn analysis(&self, c:char) -> AResult {
        let mut ans = AResult{ 
            sign: c,
            result: EnumResult::Accept, 
            output: EnumOutput::None, 
            guide: EnumGuide::Me, 
            error: EnumError::None 
        };
        if c.is_alphas()||c.is_digits()||c.is_signal(&['_']) {/*do nothing*/}
        else if c.is_enter()||c.is_space()||c.is_std_signal()||c.is_end() {
            ans.result = EnumResult::Return;
            ans.output = EnumOutput::Kword;
            ans.guide = EnumGuide::Begin;
        }else {ans.error = EnumError::UnknownSignal;}
        ans
    }
}

struct DfaInteger;
impl DfaNode for DfaInteger{
    fn analysis(&self, c:char) -> AResult {
        let mut ans = AResult{ 
            sign: c,
            result: EnumResult::Accept, 
            output: EnumOutput::None, 
            guide: EnumGuide::Me, 
            error: EnumError::None 
        };
        //println!("c={}.", c);
        if c.is_digits() {/*do nothing*/}
        else if c.is_alphas() {ans.error = EnumError::IllegalDigit;}
        else if c.is_space()||c.is_end()||c.is_std_else(&['.']) {
            ans.result = EnumResult::Return;
            ans.output = EnumOutput::Integer;
            ans.guide = EnumGuide::Begin;
        }else if c.is_signal(&['.']) {ans.guide = EnumGuide::Float;}
        else {ans.error = EnumError::UnknownSignal;}
        ans
    }
}

struct DfaFloat;
impl DfaNode for DfaFloat{
    fn analysis(&self, c:char) -> AResult {
        let mut ans = AResult{ 
            sign: c,
            result: EnumResult::Accept, 
            output: EnumOutput::None, 
            guide: EnumGuide::Me, 
            error: EnumError::None 
        };
        if c.is_digits() {/*do nothing*/}
        else if c.is_alphas()||c.is_signal(&['.']) {ans.error = EnumError::IllegalDigit;}
        else if c.is_space()||c.is_std_else(&['.'])||c.is_end() {
            ans.result = EnumResult::Return;
            ans.output = EnumOutput::Float;
            ans.guide = EnumGuide::Begin;
        }else {ans.error = EnumError::UnknownSignal;}
        ans
    }
}

struct DfaString;
impl DfaNode for DfaString{
    fn analysis(&self, c:char) -> AResult {
        let mut ans = AResult{ 
            sign: c,
            result: EnumResult::Accept, 
            output: EnumOutput::None, 
            guide: EnumGuide::Me, 
            error: EnumError::None 
        };
        if c.is_signal(&['\\']) {
            ans.result = EnumResult::AcceptAndNo;
            ans.guide = EnumGuide::TransStr;
        }else if c.is_signal(&['\"']) {
            ans.result = EnumResult::AcceptAndNo;
            ans.output = EnumOutput::Str;
            ans.guide = EnumGuide::Begin;
        }else if c.is_end() {ans.error = EnumError::UnexpectedEnd;}
        //else do nothing
        ans
    }
}

struct DfaTransStr;
impl DfaNode for DfaTransStr{
    fn analysis(&self, c:char) -> AResult {
        let mut ans = AResult{ 
            sign: c,
            result: EnumResult::Accept, 
            output: EnumOutput::None, 
            guide: EnumGuide::Str, 
            error: EnumError::None 
        };
        if c.is_end()||c.is_enter() {ans.error = EnumError::UnexpectedEnd;}
        else {
            ans.sign = match c {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                _ => c
            }
        }
        ans
    }
}

struct DfaSignal;
impl DfaNode for DfaSignal{
    fn analysis(&self, c:char) -> AResult {
        let mut ans = AResult{ 
            sign: c,
            result: EnumResult::Accept, 
            output: EnumOutput::None, 
            guide: EnumGuide::Me, 
            error: EnumError::None 
        };
        if c.is_signal(&['\"']) {
            ans.result = EnumResult::AcceptAndNo;
            ans.guide = EnumGuide::Str;
        }else if c.is_signal(&[',', '.', '(', ')', ';', '+', '=', '*', '^']) {
            ans.output = EnumOutput::Signal;
            ans.guide = EnumGuide::Begin;
        }else if c.is_signal(&['!', '<', '>']) {ans.guide = EnumGuide::MultiS1;}
        else if c.is_signal(&['-']) {ans.guide = EnumGuide::MultiS2;}
        else if c.is_signal(&['/']) {ans.guide = EnumGuide::MultiS3;}
        else {ans.error = EnumError::UnknownSignal;}
        ans
    }
}

struct DfaMultiS1;
impl DfaNode for DfaMultiS1{
    fn analysis(&self, c:char) -> AResult {
        AResult{ 
            sign: c,
            result: if c.is_signal(&['=']) {EnumResult::Accept} else {EnumResult::Return}, 
            output: EnumOutput::Signal, 
            guide: EnumGuide::Begin, 
            error: EnumError::None 
        }
    }
}

struct DfaMultiS2;
impl DfaNode for DfaMultiS2{
    fn analysis(&self, c:char) -> AResult {
        if c.is_signal(&['-']) {
            AResult{ 
                sign: c,
                result: EnumResult::Clear, 
                output: EnumOutput::None, 
                guide: EnumGuide::Comment1, 
                error: EnumError::None 
            } 
        }else {
            AResult{ 
                sign: c,
                result: EnumResult::Return, 
                output: EnumOutput::Signal, 
                guide: EnumGuide::Begin, 
                error: EnumError::None 
            }
        }
    }
}

struct DfaMultiS3;
impl DfaNode for DfaMultiS3{
    fn analysis(&self, c:char) -> AResult {
        if c.is_signal(&['*']) {
            AResult{ 
                sign: c,
                result: EnumResult::Clear, 
                output: EnumOutput::None, 
                guide: EnumGuide::Comment2, 
                error: EnumError::None 
            }
        }else {
            AResult{ 
                sign: c,
                result: EnumResult::Return, 
                output: EnumOutput::Signal, 
                guide: EnumGuide::Begin, 
                error: EnumError::None 
            }
        }
    }
}

struct DfaComment1;
impl DfaNode for DfaComment1{
    fn analysis(&self, c:char) -> AResult {
        let flag = c.is_enter();
        AResult{ 
            sign: c,
            result: if flag {EnumResult::Clear}else{EnumResult::Accept}, 
            output: EnumOutput::None, 
            guide: if flag {EnumGuide::Begin}else{EnumGuide::Me}, 
            error: EnumError::None 
        }
    }
}

struct DfaComment2;
impl DfaNode for DfaComment2{
    fn analysis(&self, c:char) -> AResult {
        let mut ans = AResult{ 
            sign: c,
            result: EnumResult::Accept, 
            output: EnumOutput::None, 
            guide: EnumGuide::Me, 
            error: EnumError::None 
        };
        if c.is_end() {ans.error = EnumError::UnexpectedEnd;}
        else if c.is_signal(&['*']) {ans.guide = EnumGuide::CheckComment;}
        ans
    }
}

struct DfaCheckComment;
impl DfaNode for DfaCheckComment{
    fn analysis(&self, c:char) -> AResult {
        let flag = c.is_signal(&['/']);
        AResult{ 
            sign: c,
            result: if flag{EnumResult::Clear}else{EnumResult::Accept}, 
            output: EnumOutput::None, 
            guide: if flag{EnumGuide::Begin}else{EnumGuide::Comment2}, 
            error: EnumError::None 
        }
    }
}

pub struct FiniteAutomaton{
    stream:String,
    nodeset:HashMap<EnumGuide, Box<DfaNode>>,
    error:(i32, EnumError)
}
impl FiniteAutomaton{
    pub fn new(ss:String) -> Self {
        let mut map:HashMap<EnumGuide, Box<DfaNode>> = HashMap::new();
        map.insert(EnumGuide::Begin, Box::new(DfaBegin{}));
        map.insert(EnumGuide::Kword, Box::new(DfaKword{}));
        map.insert(EnumGuide::Integer, Box::new(DfaInteger{}));
        map.insert(EnumGuide::Float, Box::new(DfaFloat{}));
        map.insert(EnumGuide::Str, Box::new(DfaString{}));
        map.insert(EnumGuide::TransStr, Box::new(DfaTransStr{}));
        map.insert(EnumGuide::Signal, Box::new(DfaSignal{}));
        map.insert(EnumGuide::MultiS1, Box::new(DfaMultiS1{}));
        map.insert(EnumGuide::MultiS2, Box::new(DfaMultiS2{}));
        map.insert(EnumGuide::MultiS3, Box::new(DfaMultiS3{}));
        map.insert(EnumGuide::Comment1, Box::new(DfaComment1{}));
        map.insert(EnumGuide::Comment2, Box::new(DfaComment2{}));
        map.insert(EnumGuide::CheckComment, Box::new(DfaCheckComment{}));
        FiniteAutomaton{
            stream: ss,
            nodeset: map,
            error: (0, EnumError::None)
        }
    }

    pub fn construct(&mut self) -> Vec<DfaWord> {
        let mut li:Vec<DfaWord> = vec![];
        let mut que:String = "".to_string();
        let mut node:&DfaNode = self.nodeset[&EnumGuide::Begin].borrow();

        let vect:Vec<char> = self.stream.chars().collect();
        let mut i = 0;
        while i < vect.len() {
            let AResult{sign, result, output, guide, error} = node.analysis(vect[i]);
            //println!("[{}][{}]R={}, O={}, G={}, E={}", i ,vect[i], result as i32, output as i32, guide as i32, error as i32);

            if error != EnumError::None {
                self.error = (i as i32, error);
                break;
            }else {
                match result {
                    EnumResult::Accept => {que += &format!("{}", sign);i+=1;},
                    EnumResult::AcceptAndNo => {i+=1;},
                    EnumResult::Clear => {que = "".to_string();i+=1;},
                    EnumResult::Return => {}
                }
                match output {
                    EnumOutput::Kword => {
                        let op_string = &que.to_lowercase();
                        let kword_list = [
                            "select", "from", "where", "having", "group", "by", "order", "distinct", "use", "all",
                            "create", "table", "database", "update", "alter", "delete", "insert", "into",
                            "between", "is", "null", "as", "desc", "values", "set", "help",
                            "integer", "float", "bool", "auto_increment",
                            "foreign", "key", "reference", "primary", "unique", "not_null", "default",
                            "add", "drop", "user", "adminuser", "grant", "revoke", "privileges", "on", "to",
                            "password", "with", "view"
                        ];
                        let bool_list = [
                            "true", "false"
                        ];
                        let mut var_flag = true;
                        for case in kword_list.iter() { //是否是kword
                            if op_string == case {
                                var_flag = false;
                                li.push(DfaWord::Kword(op_string.to_string()));
                                break;
                            }
                        }
                        for case in bool_list.iter() { //是否是布尔值
                            if op_string == case {
                                var_flag = false;
                                li.push(DfaWord::Bool(op_string.parse().unwrap()));
                                break;
                            }
                        }
                        if op_string == "and" {
                            var_flag = false;
                            li.push(DfaWord::Signal("&&".to_string()));
                        }else if op_string == "or" {
                            var_flag = false;
                            li.push(DfaWord::Signal("||".to_string()));
                        }else if op_string == "not" {
                            var_flag = false;
                            li.push(DfaWord::Signal("!".to_string()));
                        }else if op_string == "varchar" {
                            var_flag = false;
                            li.push(DfaWord::Kword("str".to_string()));
                        }else if op_string == "int" {
                            var_flag = false;
                            li.push(DfaWord::Kword("integer".to_string()));
                        }else if op_string == "boolean" {
                            var_flag = false;
                            li.push(DfaWord::Kword("bool".to_string()));
                        }
                        if var_flag {li.push(DfaWord::Var(op_string.to_string()));}
                    },
                    EnumOutput::Integer => {li.push(DfaWord::Integer(que.to_string().parse().unwrap()));},
                    EnumOutput::Float => {li.push(DfaWord::Float(que.to_string().parse().unwrap()));},
                    EnumOutput::Str => {li.push(DfaWord::Str(que.to_string()));},
                    EnumOutput::Signal => {li.push(DfaWord::Signal(que.to_string()));}
                    _ => {}
                }
                if output != EnumOutput::None {que = "".to_string();}
                if guide != EnumGuide::Me {
                    node = self.nodeset[&guide].borrow();
                }
            }
        }
        if self.error.0 == 0 {return li;}
        else {return vec![];}
    }

    pub fn get_error(&self) -> &(i32, EnumError) {
        &self.error
    }

    pub fn get_error_string(&self) -> Option<String> {
        if self.error.0 > 0 {
            Option::Some(match self.error.1 {
                EnumError::None => format!("No Error."),
                EnumError::UnknownSignal => format!("Unknown Signal in {}.", self.error.0),
                EnumError::IllegalDigit => format!("Illegal Digit in {}.", self.error.0),
                EnumError::UnexpectedEnd => format!("Unexpected End."),
                EnumError::ExternalError => format!("External Error in {}.", self.error.0)
            })
        }else{
            Option::None
        }
    }
}