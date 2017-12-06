extern crate serde_json;
use std::cmp::Ordering;
use std::fs::{File, OpenOptions, remove_file};
use std::io::{Read, Write, Seek, SeekFrom};
use std::mem::{transmute};
use std::collections::HashMap;
use self::serde_json::{Value, Number, Map};
use super::database::{Database};
use super::super::analyse::dfa::{DfaWord};
use super::super::syntax::structures::{ExpressionType, SelectSyntax, Expression, Switch};

const PAGE_SIZE:usize = 64 * 1024; // 64K

//= 存储数据库行为描述和文件划分的结构 =======================================
/*
    该文件的作用是存储数据库信息与数据库存储信息。
    需要存储数据库的名称等配置信息。
    需要存储数据库的save文件的分页信息。
    本存储结构采用内存同步结构。
*/
pub struct ConfigFile {
    pub name:String,
    filepath:String,
    basepath:String,
    pub pages:Vec<PageType>,
    pub table_pages: HashMap<String, Vec<usize>>,
    pub tables: HashMap<String, TableConfig>,
    pub views: HashMap<String, SelectSyntax>
}
impl ConfigFile {
    pub fn new(basepath:String, name:String) -> Self {
        let filepath = basepath.to_string() + name.as_str() + ".dba";
        //println!("open file {}", filepath);
        let mut f = OpenOptions::new().create(true).read(true).write(true).open(filepath.to_string()).unwrap();
        {
            //开一下存储文件试试。
            let fs = OpenOptions::new().create(true).read(true).write(true).open(basepath.to_string() + name.as_str() + ".db").unwrap();
        }
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();;
        let v: Value = match serde_json::from_str(&s) {
            Ok(ok) => ok,
            Err(..) => Value::Null
        };
        if let Value::Object(ref map) = v {
            //读分页
            let pages = if let Value::Array(ref arr) = map["pages"] {
                let mut nw:Vec<PageType> = Vec::with_capacity(arr.len());
                for i in arr.iter() {
                    if let &Value::String(ref s) = i {
                        nw.push(PageType::from_string(s.to_string()));
                    }else {
                        panic!("WRONG EXTERNAL TYPE: PAGE TYPE Is NOT A STRING.");
                    }
                }
                nw
            }else{panic!("Wrong config type.");};
            //从分页加载每个表的分页
            let mut table_pages:HashMap<String, Vec<usize>> = HashMap::new();
            for (i, m) in pages.iter().enumerate() {
                match m {
                    &PageType::Data(ref s) => {
                        if table_pages.contains_key(s.as_str()) {
                            table_pages.get_mut(s.as_str()).unwrap().push(i);
                        }else{
                            table_pages.insert(s.to_string(), vec![i]);
                        }
                    }
                }
            }
            //读表配置
            let tables = if let Value::Object(ref fl) = map["tables"] {
                let mut nw:HashMap<String, TableConfig> = HashMap::new();
                for (k, v) in fl.iter() {
                    nw.insert(k.to_string(), TableConfig::from_json(v));
                }nw
            }else{panic!("Wrong config type.");};
            //读视图配置
            let views = if let Value::Object(ref fl) = map["views"] {
                let mut nw:HashMap<String, SelectSyntax> = HashMap::new();
                for (k, v) in fl.iter() {
                    nw.insert(k.to_string(), SelectSyntax::from_json(v));
                }nw
            }else{panic!("Wrong config type.");};

            return ConfigFile {
                name: if let Value::String(ref s) = map["name"] {s.to_string()}else{name},
                filepath: filepath,
                basepath: basepath,
                pages: pages,
                table_pages: table_pages,
                tables: tables,
                views: views
            };
        }else if let Value::Null = v {
            return ConfigFile {
                name: name,
                filepath: filepath,
                basepath: basepath,
                pages: Vec::new(),
                table_pages: HashMap::new(),
                tables: HashMap::new(),
                views: HashMap::new()
            };
        }else{
            return ConfigFile {
                name: name,
                filepath: filepath,
                basepath: basepath,
                pages: Vec::new(),
                table_pages: HashMap::new(),
                tables: HashMap::new(),
                views: HashMap::new()
            };
        }
    }
    pub fn save(&self) {
        let mut map:HashMap<String, Value> = HashMap::new();
        map.insert("name".to_string(), Value::String(self.name.to_string()));
        let mut arr = Vec::new();
        for i in self.pages.iter() {arr.push(Value::String(i.to_string()));}
        map.insert("pages".to_string(), Value::Array(arr));
        let mut tables = Map::new();
        for (k ,v) in self.tables.iter() {tables.insert(k.to_string(), v.to_json());}
        map.insert("tables".to_string(), Value::Object(tables));
        let mut views = Map::new();
        for (k, v) in self.views.iter() {views.insert(k.to_string(), v.to_json());}
        map.insert("views".to_string(), Value::Object(views));

        let js = serde_json::to_string(&map).unwrap();

        let mut f = OpenOptions::new().create(true).truncate(true).write(true).open(self.filepath.to_string()).unwrap();
        f.write_all(js.as_bytes()).unwrap();
    }
    pub fn delete_file(&self) {
        remove_file(self.basepath.to_string() + self.name.as_str() + ".db").unwrap();
        remove_file(self.filepath.to_string()).unwrap();
    }
    pub fn session(&mut self) -> Database {
        Database{
            file: SaveFile::new(self.basepath.to_string() + self.name.as_str() + ".db"),
            conf: self
        }
    }
}
impl SelectSyntax {
    fn from_json(v:&Value) -> SelectSyntax {
        if let &Value::Object(ref map) = v {
            let distinct = if let Option::Some(some) = map.get("distinct") {
                if let &Value::Bool(b) = some {b}else{false}
            }else{false};

            let froms = if let Option::Some(some) = map.get("froms") {
                if let &Value::Object(ref map) = some{
                    let mut froms = HashMap::new();
                    for (k, v) in map.iter() {
                        match v {
                            &Value::String(ref s) => {froms.insert(k.to_string(), Switch::One(s.to_string()));},
                            &Value::Array(..) => {froms.insert(k.to_string(), Switch::Two(SelectSyntax::from_json(v)));},
                            _ => {}
                        }
                    }froms
                }else{HashMap::new()}
            }else{HashMap::new()};

            let goals = if let Option::Some(some) = map.get("goals") {
                if let &Value::Array(ref arr) = some {
                    let mut goals = vec![];
                    for i in arr.iter() {
                        if let &Value::Array(ref pair) = i {
                            let p1 = if let Value::String(ref s) = pair[0] {s}else{panic!("Wrong type.")};
                            let p2 = Expression::from_json(&pair[1]);
                            goals.push((p1.to_string(), p2));
                        }
                    }goals
                }else{vec![]}
            }else{vec![]};
            let orders = if let Option::Some(some) = map.get("orders") {
                if let &Value::Array(ref arr) = some {
                    let mut orders = vec![];
                    for i in arr.iter() {
                        if let &Value::Array(ref pair) = i {
                            let p1 = if let Value::String(ref s) = pair[0] {s}else{panic!("Wrong type.")};
                            let p2 = if let Value::Bool(b) = pair[1] {b}else{false};
                            orders.push((p1.to_string(), p2));
                        }
                    }orders
                }else{vec![]}
            }else{vec![]};
            let wheres = if let Option::Some(some) = map.get("wheres") {
                Expression::from_json(some)
            }else{Expression::empty()};
            SelectSyntax {
                distinct: distinct,
                froms: froms,
                goals: goals,
                orders: orders,
                wheres: wheres
            }
        }else{panic!("Wrong config type.");}
        
    }
    fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("distinct".to_string(), Value::Bool(self.distinct));

        let mut froms = Map::new();
        for (k, v) in self.froms.iter() {
            froms.insert(k.to_string(), match v{
                &Switch::One(ref s) => Value::String(s.to_string()),
                &Switch::Two(ref exp) => exp.to_json() // array
            });
        }
        map.insert("froms".to_string(), Value::Object(froms));

        let mut goals = Vec::new();
        for &(ref k, ref v) in self.goals.iter() {
            goals.push(Value::Array(vec![Value::String(k.to_string()), v.to_json()]));
        }
        map.insert("goals".to_string(), Value::Array(goals));
        map.insert("wheres".to_string(), self.wheres.to_json());

        let mut orders = Vec::new();
        for &(ref k, ref v) in self.orders.iter() {
            orders.push(Value::Array(vec![Value::String(k.to_string()), Value::Bool(*v)]));
        }
        map.insert("orders".to_string(), Value::Array(orders));
        Value::Object(map)
    }
}
impl Expression {
    fn from_json(v:&Value) -> Expression {
        if let &Value::Object(ref map) = v {
            let li = if let Value::Array(ref arr) = map["li"] {
                let mut li = vec![];
                for i in arr.iter() {
                    if let &Value::String(ref s) = i {
                        let tp = &s[..7].trim().to_string();
                        li.push(match tp.as_str() {
                            "Kword" => ExpressionType::Kword(s[7..].to_string()),
                            "Var" => {
                                let v:Vec<&str> = s[7..].split('.').collect();
                                let vv:Vec<String> = v.iter().map(|i|i.to_string()).collect();
                                ExpressionType::Var(vv)
                            },
                            "Bool" => ExpressionType::Bool(s[7..].parse().unwrap()),
                            "Integer" => ExpressionType::Integer(s[7..].parse().unwrap()),
                            "Float" => ExpressionType::Float(s[7..].parse().unwrap()),
                            "Str" => ExpressionType::Str(s[7..].to_string()),
                            "Signal" => ExpressionType::Signal(s[7..].to_string()),
                            _ => {panic!(format!("Wrong type config."))}
                        });
                    }
                }
                li
            }else{panic!("Wrong type.")};
            let set = if let Value::String(ref s) = map["setence"] {
                s.to_string()
            }else{panic!("Wrong type.")};
            Expression{li:li, setence: set}
        }else{panic!("Wrong type.")}
        
    }
    fn to_json(&self) -> Value {
        let mut arr = vec![];
        for i in self.li.iter() {
            arr.push(Value::String(match i {
                &ExpressionType::Kword(ref s) => format!("kword  {}", s),
                &ExpressionType::Var(ref v) => format!("Var    {}", v.join(".").to_string()),
                &ExpressionType::Bool(b) => format!("Bool   {}", b),
                &ExpressionType::Integer(i) => format!("Integer{}", i),
                &ExpressionType::Float(f) => format!("Float  {}", f),
                &ExpressionType::Str(ref s) => format!("Str    {}", s),
                &ExpressionType::Signal(ref s) => format!("Signal {}", s)
            }));
        }
        let mut map = Map::new();
        map.insert("li".to_string(), Value::Array(arr));
        map.insert("setence".to_string(), Value::String(self.setence.to_string()));
        Value::Object(map)
    }
}
pub enum PageType {
    Data(String)  //数据页
}
impl PageType {
    pub fn to_string(&self) -> String {
        match self {
            &PageType::Data(ref s) => format!("data:{}", s)
        }
    }
    pub fn from_string(s:String) -> Self {
        if s.starts_with("data:") {
            PageType::Data(s[5..].to_string())
        }else{
            panic!("WRONG EXTERNAL TYPE: PAGE TYPE ERROR.")
        }
    }
}




//= 表配置 =============================================
pub struct TableConfig {
    pub name: String,  //表名
    pub fields: Vec<FieldConfig>,  // 字段列表
    pub primary: Vec<String>,  // 特别标记主键
    pub auto_config: HashMap<String, usize>,  //用来记录自增属性的自增位置
    pub foreign: HashMap<String, ForeignConfig>,  //记录外键列表
    pub count: usize  //记录数量
}
impl TableConfig {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        // insert name
        map.insert("name".to_string(), Value::String(self.name.to_string()));
        // insert fields
        let mut fields = Vec::new();
        for field in self.fields.iter() {
            fields.push(field.to_json());
        }
        map.insert("fields".to_string(), Value::Array(fields));
        //insert foreign
        let mut foreigns = Map::new();
        for (k, v) in &self.foreign {
            foreigns.insert(k.to_string(), v.to_json());
        }
        map.insert("foreign".to_string(), Value::Object(foreigns));
        //insert auto
        let mut auto = Map::new();
        for (k, v) in &self.auto_config {
            auto.insert(k.to_string(), Value::Number(Number::from_f64(*v as f64).unwrap()));
        }
        map.insert("auto_config".to_string(), Value::Object(auto));
        //insert count
        map.insert("count".to_string(), Value::Number(Number::from_f64(self.count as f64).unwrap()));
        //return result
        Value::Object(map)
    }
    pub fn from_json(v:&Value) -> Self {
        if let &Value::Object(ref map) = v {
            let name = if let Option::Some(ref s) = map.get("name") {
                if let &Value::String(ref ss) = *s {
                    ss.to_string()
                }else{"".to_string()}
            }else{"".to_string()};
            let count = if let Option::Some(ref s) = map.get("count") {
                if let &Value::Number(ref n) = *s {
                    n.as_f64().unwrap() as usize
                }else{0}
            }else{0};
            let fields = if let Option::Some(ref arrs) = map.get("fields") {
                if let &Value::Array(ref arr) = *arrs {
                    let mut nw = Vec::new();
                    for i in arr.iter() {
                        nw.push(FieldConfig::from_json(i));
                    }
                    nw
                }else{vec![]}
            }else{vec![]};
            let mut primary = Vec::new();
            for i in fields.iter() { if i.primary {primary.push(i.name.to_string());} }
            let foreign = if let Option::Some(ref oob) = map.get("foreign") {
                if let &Value::Object(ref ob) = *oob {
                    let mut map = HashMap::new();
                    for (k, v) in ob.iter() {
                        map.insert(k.to_string(), ForeignConfig::from_json(v));
                    }
                    map
                }else{HashMap::new()}
            }else{HashMap::new()};
            let auto_config = if let Option::Some(ref oob) = map.get("auto_config") {
                if let &Value::Object(ref ob) = *oob {
                    let mut map = HashMap::new();
                    for (k, v) in ob.iter() {
                        if let &Value::Number(ref num) = v {
                            map.insert(k.to_string(), num.as_f64().unwrap() as usize);
                        }
                    }
                    map
                }else{HashMap::new()}
            }else{HashMap::new()};
            Self {
                name: name,
                fields: fields,
                auto_config: auto_config,
                foreign: foreign,
                primary: primary,
                count: count
            }
        }else {panic!("Wrong config type.");}
    }
    pub fn get_template(&self) -> Data {
        let mut li = vec![];
        for i in self.fields.iter() {
            li.push(i.t.get_dataitem());
        }
        Data {li: li}
    }
}
pub struct ForeignConfig {
    pub field: String,  // 在本表中的键名
    pub foreign_table: String,  // 外表表名
    pub foreign_field: String,  // 在外表中的field
    pub delete_action: ForeignType  // 删除操作类型
}
impl ForeignConfig {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("field".to_string(), Value::String(self.field.to_string()));
        map.insert("foreign_table".to_string(), Value::String(self.foreign_table.to_string()));
        map.insert("foreign_field".to_string(), Value::String(self.foreign_field.to_string()));
        map.insert("delete_action".to_string(), Value::String(self.delete_action.to_string()));
        Value::Object(map)
    }
    pub fn from_json(v:&Value) -> Self {
        if let &Value::Object(ref map) = v {
            Self {
                field: if let Option::Some(t) = map.get("field") {
                    if let &Value::String(ref s) = t {s.to_string()}else{panic!("Wrong config type.");}
                }else{panic!("No name of field.");},
                foreign_table: if let Option::Some(t) = map.get("foreign_table") {
                    if let &Value::String(ref s) = t {s.to_string()}else{panic!("Wrong config type.");}
                }else{panic!("No foreign_table.");},
                foreign_field: if let Option::Some(t) = map.get("foreign_field") {
                    if let &Value::String(ref s) = t {s.to_string()}else{panic!("Wrong config type.");}
                }else{panic!("No foreign_field.");},
                delete_action: if let Option::Some(t) = map.get("delete_action") {
                    if let &Value::String(ref s) = t {ForeignType::from_string(s)}else{panic!("Wrong config type.");}
                }else{panic!("No name of field.");},
            }
        }else{panic!("Wrong config type.");}
    }
}

pub struct FieldConfig {
    pub name: String,
    pub t: FieldType,
    pub unique: bool,
    pub primary: bool,
    pub not_null: bool,
    pub default: Option<DataItem>,
    pub auto_inc: bool
}
impl FieldConfig {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("name".to_string(), Value::String(self.name.to_string()));
        map.insert("type".to_string(), Value::String(self.t.to_string()));
        map.insert("unique".to_string(), Value::Bool(self.unique));
        map.insert("primary".to_string(), Value::Bool(self.primary));
        map.insert("not_null".to_string(), Value::Bool(self.not_null));
        map.insert("auto_inc".to_string(), Value::Bool(self.auto_inc));
        map.insert("default".to_string(), match self.default {
            Option::None => Value::Null,
            Option::Some(ref t) => match t {
                &DataItem::Str(_, ref s) => Value::String(s.to_string()),
                &DataItem::Bool(b) => Value::Bool(b),
                &DataItem::Integer(i) => Value::Number(Number::from_f64(i as f64).unwrap()),
                &DataItem::Float(f) => Value::Number(Number::from_f64(f).unwrap())
            }
        });
        Value::Object(map)
    }
    pub fn from_json(v:&Value) -> Self {
        if let &Value::Object(ref map) = v {
            //优先处理type。
            let t = if let Option::Some(t) = map.get("type") {
                if let &Value::String(ref s) = t {FieldType::from_string(s)}else{panic!("Wrong config type.");}
            }else{panic!("No name of field.");};
            Self{
                name: if let Option::Some(t) = map.get("name") {
                    if let &Value::String(ref s) = t {s.to_string()}else{panic!("Wrong config type.");}
                }else{panic!("No name of field.");},
                unique: if let Option::Some(t) = map.get("unique") {
                    if let &Value::Bool(s) = t {s}else{false}
                }else{false},
                primary: if let Option::Some(t) = map.get("primary") {
                    if let &Value::Bool(s) = t {s}else{false}
                }else{false},
                not_null: if let Option::Some(t) = map.get("not_null") {
                     if let &Value::Bool(s) = t {s}else{false}
                }else{false},
                auto_inc: if let Option::Some(t) = map.get("auto_inc") {
                    if let &Value::Bool(s) = t {s}else{false}
                }else{false},
                default: if let Option::Some(some) = map.get("default") {
                    match some {
                        &Value::Bool(s) => Option::Some(DataItem::Bool(s)),
                        &Value::String(ref s) => Option::Some(DataItem::Str(0, s.to_string())),
                        &Value::Number(ref n) => Option::Some(match t{
                            FieldType::Float => DataItem::Float(n.as_f64().unwrap()),
                            FieldType::Integer => DataItem::Integer(n.as_i64().unwrap()),
                            _ => {panic!("Wrong config type.");}
                        }),
                        _ => {Option::None}
                    }
                }else{Option::None},
                t: t
            }
        }else {panic!("Wrong config type.");}
    }
}
#[derive(Hash, Eq, Copy, Clone)]
pub enum FieldType {
    Integer,
    Float,
    Bool,
    Str(usize)
}
impl PartialEq for FieldType {
    fn eq(&self, other: &FieldType) -> bool {
        match self {
            &FieldType::Integer => if let &FieldType::Integer = other {true}else{false},
            &FieldType::Float => if let &FieldType::Float = other {true}else{false},
            &FieldType::Bool => if let &FieldType::Bool = other {true}else{false},
            &FieldType::Str(_) => if let &FieldType::Str(_) = other {true}else{false},
        }
    }
}
impl FieldType {
    pub fn to_string(&self) -> String {
        match self {
            &FieldType::Integer => "integer".to_string(),
            &FieldType::Float => "float".to_string(),
            &FieldType::Bool => "bool".to_string(),
            &FieldType::Str(u) => format!("str:{}", u)
        }
    }
    pub fn from_string(s:&str) -> Self {
        match s {
            "integer" => FieldType::Integer,
            "float" => FieldType::Float,
            "bool" => FieldType::Bool,
            _ => {
                if s.starts_with("str:") {
                    FieldType::Str(s[4..].parse().unwrap())
                }else{
                    panic!("Wrong config value.");
                }
            }
        }
    }
    pub fn get_dataitem(&self) -> DataItem {
        match self {
            &FieldType::Integer => DataItem::Integer(0),
            &FieldType::Float => DataItem::Float(0.0),
            &FieldType::Bool => DataItem::Bool(false),
            &FieldType::Str(u) => DataItem::Str(u, "".to_string())
        }
    }
}
pub enum ForeignType {
    Cascade,
    SetNull,
    Restrict
}
impl ForeignType {
    pub fn to_string(&self) -> String {
        match self {
            &ForeignType::Cascade => "cascade",
            &ForeignType::SetNull => "setnull",
            &ForeignType::Restrict => "restrict"
        }.to_string()
    }
    pub fn from_string(s:&str) -> Self {
        match s {
            "cascade" => ForeignType::Cascade,
            "setnull" => ForeignType::SetNull,
            "restrict" => ForeignType::Restrict,
            _ => {panic!("Wrong config value.");}
        }
    }
}

//= 存储数据库文件内容的结构 ==============================
/*
    该文件的作用是储存数据库的表的内容。
    采用分页模式。以PAGE_SIZE为页大小，将表数据按页插入。
    页的划分存储在config文件中。
*/
pub struct SaveFile{
    file:File
}
impl SaveFile {
    pub fn new(filepath:String) -> Self {
        Self {
            file: OpenOptions::new().write(true).read(true).create(true).open(filepath.to_string()).unwrap()
        }
    }
    fn get_start_seek(page:&[usize], newpage:usize, seek:usize, len:usize) -> Vec<(usize, usize, usize, usize)> {
        //返回值：(页号, 起始文件指针, 数据字节位置, io长度)
        let get_page = |p|{
            if p < page.len() {page[p]}
            else {p - page.len() + newpage}
        };
        let mut v:Vec<(usize, usize, usize, usize)> = Vec::new();
        let location = len * seek; // 虚拟的文件指针的位置
        //处理开头。
        let mut page_i = seek * len / PAGE_SIZE;
        loop {
            let real_page = get_page(page_i); // 获得该页的实际页号。

            let page_real_begin = PAGE_SIZE * real_page; // 该页的实际页首指针。
            //let page_real_end = page_real_begin + PAGE_SIZE; // 该页的实际页尾指针。

            let page_begin = PAGE_SIZE * page_i; // 虚拟页的页首
            let page_end = page_begin + PAGE_SIZE; // 虚拟页的页尾

            if location + len < page_begin {break;} // 如果数据位小于虚拟页首表示已经没有需要处理的数据了。

            let data_begin = if location >= page_begin {0}else{page_begin - location}; // 应处理的数据首
            let data_end = if location + len < page_end {len}else{page_end - location}; // 应处理的数据尾

            let seek_begin = if location >= page_begin {location - page_begin}else{0};
            //let seek_end = if location + len < page_end {location + len - page_begin}else{0};
            
            v.push((real_page, seek_begin + page_real_begin, data_begin, data_end - data_begin));
            page_i += 1;
        }
        v
    }
    pub fn write(&mut self, page:&[usize], newpage:usize, seek: usize, d:&Data) -> Option<usize> {
        //给出的页列表会按照顺序依次往下io。后一个页号需要作为新页的标记，从这个标记开始可以随意创建新页。
        //seek代表的不是文件指针的字节位置，而是在当前Data的长度下，记录的条目位置。
        //返回的Some是在创建了新页的情况下，最后一个页的页号。
        //println!("WRITE");
        let mut t = Vec::new();
        d.to_bytes(&mut t);
        let plist = SaveFile::get_start_seek(page, newpage, seek, d.len());
        let mut ret:Option<usize> = Option::None;
        for &(i, pb, db, l) in &plist {
            self.file.seek(SeekFrom::Start(pb as u64)).unwrap();
            //println!("i={}, pb in [{}, {}], db in [{}, {}], len={}", i, pb, pb+l, db, db+l, t.len());
            self.file.write(&t[db..db + l]).unwrap();
            if i >= newpage {ret = Option::Some(i);}
        }
        //self.file.write(&t[..]);
        ret
    }
    pub fn read(&mut self, page:&[usize], newpage:usize, seek: usize, d:&mut Data) -> Option<usize> {
        //给出的页列表会按照顺序依次往下io。后一个页号需要作为新页的标记，从这个标记开始可以随意创建新页。
        //seek代表的不是文件指针的字节位置，而是在当前Data的长度下，记录的条目位置。
        //返回的Some是在创建了新页的情况下，最后一个页的页号。
        //println!("read[{}]", seek);
        //println!("READ");
        const PART_SIZE:usize = 64;
        let len = d.len();
        let mut t:Vec<u8> = Vec::new();
        let plist = SaveFile::get_start_seek(page, newpage, seek, len);
        let mut ret:Option<usize> = Option::None;
        for &(i, pb, _, l) in &plist {
            //println!("  plist[i={}, pb={}, db={}, l={}]", i, pb, db, l);
            let mut part_t:Vec<u8> = Vec::new();
            self.file.seek(SeekFrom::Start(pb as u64)).unwrap();
            while part_t.len() < l {
                let mut part:[u8; PART_SIZE] = [0_u8; PART_SIZE];
                match self.file.read(&mut part) {
                    Result::Ok(n) => {
                        //println!("      read {}", n);
                        part_t.extend_from_slice(&part[..n]);
                        if n < PART_SIZE {break;}
                    },
                    Result::Err(_) => {
                        //println!("      error occured.");
                        break;
                    }
                }
            }
            t.extend_from_slice(&part_t[..l]);
            if i >= newpage {ret = Option::Some(i);}
            //println!("i={}, pb in [{}, {}], db in [{}, {}], len={}", i, pb, pb+l, db, db+l, part_t.len());
        }
        Data::from_bytes(&t[0..len], d);
        ret
    }
}

//= 单条记录对象 ================================================
pub enum DataItem {
    Integer(i64),
    Float(f64),
    Str(usize, String),
    Bool(bool)
}
impl DataItem {
    pub fn cmp(&self, d:&DataItem) -> Result<Ordering, ()> {
        match self {
            &DataItem::Integer(i) => if let &DataItem::Integer(j) = d {Result::Ok(i.cmp(&j))}else{Result::Err(())},
            &DataItem::Float(i) => if let &DataItem::Float(j) = d {Result::Ok(
                if i == j {Ordering::Equal}else if i < j {Ordering::Less}else{Ordering::Greater}
            )}else{Result::Err(())},
            &DataItem::Str(_, ref i) => if let &DataItem::Str(_, ref j) = d {Result::Ok(i.cmp(j))}else{Result::Err(())},
            &DataItem::Bool(i) => if let &DataItem::Bool(j) = d {Result::Ok(
                if (i && j)||(!i&&!j) {Ordering::Equal}else if i&&!j {Ordering::Less}else{Ordering::Greater}
            )}else{Result::Err(())}
        }
    }
    pub fn eq(&self, d:&DataItem) -> bool {
        match self {
            &DataItem::Integer(i) => if let &DataItem::Integer(j) = d {i==j}else{false},
            &DataItem::Float(i) => if let &DataItem::Float(j) = d {i==j}else{false},
            &DataItem::Str(_ ,ref i) => if let &DataItem::Str(_, ref j) = d {i.to_string()==j.to_string()}else{false},
            &DataItem::Bool(i) => if let &DataItem::Bool(j) = d {i==j}else{false}
        }
    }
    fn to_bytes(&self, ret:&mut Vec<u8>) {
        match self {
            &DataItem::Integer(i) => {
                let nw:[u8; 8] = unsafe{transmute(i)};
                *ret = nw.to_vec();
            },
            &DataItem::Float(f) => {
                let nw:[u8; 8] = unsafe{transmute(f)};
                *ret = nw.to_vec();
            },
            &DataItem::Str(l, ref s) => {
                
                //println!("[{}]", s);
                let nw = s.as_bytes();
                //print!("[");for ii in nw.iter() {print!("{}, ", ii);}println!("]");
                *ret = nw.to_vec();
                let mut need_space = l * 4 - nw.len();
                while need_space > 0 {
                    ret.push(0);
                    need_space -=1;
                } 
            }
            &DataItem::Bool(b) => {
                let nw:[u8; 1] = unsafe{transmute(b)};
                *ret = nw.to_vec();
            }
        };
    }
    fn from_bytes(from:&[u8], goal:&mut DataItem) {
        match *goal {
            DataItem::Integer(ref mut i) => {
                let mut od = [0_u8; 8];
                for i in 0..8 { od[i] = from[i];}
                *i = unsafe{transmute(od)};
            },
            DataItem::Float(ref mut f) => {
                let mut od = [0_u8; 8];
                for i in 0..8 { od[i] = from[i];}
                *f = unsafe{transmute(od)};
            },
            DataItem::Str(_, ref mut s) => {
                //必须对\0做一个特殊处理，截断下来。
                let mut end = from.len();
                while end > 0 {
                    if from[end-1] != 0 {
                        break;
                    }
                    end -= 1;
                }
                *s = String::from_utf8(from[..end].to_vec()).unwrap();
                //print!("[");for ii in from[..end].iter() {print!("{}, ", ii);}println!("]");
                //println!("[{}]", s);
            },
            DataItem::Bool(ref mut b) => {
                let mut od = [0_u8; 1];
                for i in 0..1 {od[i] = from[i];}
                *b = unsafe{transmute(od)};
            }
        }
    }
    pub fn len(&self) -> usize {
        match self {
            &DataItem::Integer(..) => 8,
            &DataItem::Float(..) => 8,
            &DataItem::Str(l, _) => l * 4,
            &DataItem::Bool(..) => 1
        }
    }
    pub fn copy(&self) -> Self {
        match self {
            &DataItem::Integer(i) => DataItem::Integer(i),
            &DataItem::Float(f) => DataItem::Float(f),
            &DataItem::Bool(b) => DataItem::Bool(b),
            &DataItem::Str(u, ref s) => DataItem::Str(u, s.to_string())
        }
    }
    pub fn from_dfa(d:&DfaWord, goal:&mut DataItem){
        match goal {
            &mut DataItem::Integer(ref mut i) => {
                if let &DfaWord::Integer(value) = d {*i=value;}
            },
            &mut DataItem::Float(ref mut f) => {
                if let &DfaWord::Float(value) = d {*f=value;}
            },
            &mut DataItem::Str(_, ref mut s) => {
                if let &DfaWord::Str(ref value) = d {*s=value.to_string();}
            },
            &mut DataItem::Bool(ref mut b) => {
                if let &DfaWord::Bool(value) = d {*b=value;}
            }
        }
    }
    pub fn to_expt(&self) -> ExpressionType {
        match self {
            &DataItem::Integer(i) => ExpressionType::Integer(i),
            &DataItem::Float(f) => ExpressionType::Float(f),
            &DataItem::Bool(b) => ExpressionType::Bool(b),
            &DataItem::Str(_, ref s) => ExpressionType::Str(s.to_string())
        }
    }
    pub fn from_expt(e:&ExpressionType) -> Self {
        match e {
            &ExpressionType::Integer(i) => DataItem::Integer(i),
            &ExpressionType::Float(f) => DataItem::Float(f),
            &ExpressionType::Bool(b) => DataItem::Bool(b),
            &ExpressionType::Str(ref s) => DataItem::Str(0, s.to_string()),
            _ => {panic!("Wrong type.")}
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            &DataItem::Integer(i) => i.to_string(),
            &DataItem::Float(f) => f.to_string(),
            &DataItem::Str(_, ref s) => s.to_string(),
            &DataItem::Bool(b) => b.to_string()
        }
    }
}
pub struct Data {
    pub li:Vec<DataItem>
}
impl Data {
    pub fn new(li:Vec<DataItem>) -> Self {
        Self {li: li}
    }
    pub fn len(&self) -> usize {
        //取得一条数据记录的定长。
        let mut ret:usize = 0;
        for i in &self.li {
            ret += i.len()
        }
        ret
    }
    pub fn to_bytes(&self, ret:&mut Vec<u8>) {
        ret.clear();
        for i in &self.li {
            let mut part:Vec<u8> = Vec::new();
            //print!("[{}]", i.to_string());
            i.to_bytes(&mut part);
            //print!("[");for ii in part.iter() {print!("{}", ii);}print!("]");
            ret.extend_from_slice(&part[..]);
        }
    }
    pub fn from_bytes(from:&[u8], goal:&mut Data) {
        let mut index = 0;
        for i in &mut goal.li {
            let l = i.len();
            let v = &from[index..index+l];
            //print!("[");for ii in v.iter() {print!("{}", ii);}print!("]");
            index += l;
            DataItem::from_bytes(v, i);
            //print!("[{}]", i.to_string());
        }
    }
    pub fn copy(&self) -> Self {
        let mut li = Vec::with_capacity(self.li.len());
        for i in self.li.iter(){li.push(i.copy());}
        Self{
            li: li
        }
    }
    pub fn to_string(&self) -> String {
        let mut ret = "(".to_string();
        for (i, d) in self.li.iter().enumerate() {
            if i > 0 {ret += ", ";}
            ret += d.to_string().as_str();
        }
        ret += ")";
        ret
    }
    pub fn eq(&self, d:&Data) -> bool {
        if d.li.len() != self.li.len() {
            return false;
        }
        let len = d.li.len();
        for i in 0..len {
            if !self.li[i].eq(&d.li[i]) {
                return false;
            }
        }
        return true;
    }
}