use std::collections::HashMap;
use super::structures::{SelectSyntax, Switch, Expression, Syntax};
use super::super::analyse::dfa::{DfaWord};
//语法树相关构件。
pub struct AResult{
    pub result:EnumResult,  //对于正在处理的单词做如何处理
    pub action:Vec<String>,  //发送给语法树构造器的构造指令
    pub guide: String,   //发送给自动机的导向指令
    pub error:EnumError  //错误码
}
pub trait DfaNode {
    fn analysis(&self, w:&DfaWord) -> AResult;
    fn analysis_array(&self, w:&[DfaWord], begin:i32, end:&mut i32) -> AResult;
    fn allow_array(&self) -> bool;
}

#[derive(Hash, Eq, Copy, Clone)]
pub enum EnumError {
    None,
    SyntaxError,
    UnknownStart,
    IllegalSignal
}
impl PartialEq for EnumError {
    fn eq(&self, other: &EnumError) -> bool {
        *self as i32 == *other as i32
    }
}

#[derive(Hash, Eq, Copy, Clone)]
pub enum EnumResult {
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

pub trait Tree {
    fn construct(&mut self, li:&[DfaWord]) -> Box<Syntax>;
    fn get_error(&self) -> &(i32, EnumError);
}