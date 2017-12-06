use std::fs::{OpenOptions};
use std::collections::HashMap;
use std::io::{Read};
// 与数据库软件配置信息有关的内容

pub struct Config{
    pub database: String,
    pub systembase: String
}
impl Config{
    pub fn load(filepath:&str) -> Self {
        let mut f = OpenOptions::new().read(true).open(filepath).unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        let lines = s.lines();
        let mut map:HashMap<String, String> = HashMap::new();
        for line in lines {
            let s:Vec<&str> = line.split('=').collect();
            if s.len() >= 2 {
                map.insert(s[0].to_string(), s[1].to_string());
            }
        }
        Self {
            database: if let Option::Some(ref s) = map.get("database") {s}else{"database/"}.to_string(),
            systembase: if let Option::Some(ref s) = map.get("systembase") {s}else{"system"}.to_string()
        }
    }
}