use super::save::{
    ConfigFile, SaveFile, DataItem, TableConfig, Data, PageType,
    FieldConfig, ForeignConfig, FieldType, ForeignType
};
use std::io::{Write};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use super::super::syntax::structures::{
    Switch, Expression, ExpressionType, SelectSyntax, InsertSyntax, DeleteSyntax, UpdateSyntax,
    CreateTableSyntax, AlterTableSyntax, DropTableSyntax, HelpSyntax,
    CreateViewSyntax, DropViewSyntax
};
use super::super::analyse::dfa::{DfaWord};
//= 工具 ==============================================
fn has<T, F>(v:&Vec<T>, t:F) -> bool where F: Fn(&T) -> bool {
    for i in v.iter() {
        if t(i) {return true;}
    }
    return false;
}

//= 数据库会话实体 =====================================
pub struct Database<'t> {
    pub conf: &'t mut ConfigFile,
    pub file: SaveFile
}
impl<'t> Database<'t> {
    fn get_table_sub(&mut self, syntax:&SelectSyntax) -> DResult {
        // for &(ref i,ref j) in syntax.goals.iter() {
        //     println!("GOAL[{}]:{}", i, j.to_string());
        // }
        /*select的语序：
            1. 提取froms的名单。
            2. 构造笛卡儿积
            3. 按照where的条件过滤记录
            4. 按照order的条件执行排序
            5. 按照goal的表达式返回列
            6. 如果有必要，就去重
        */
        // 1.
        let mut origin:Vec<(String, DTable)> = Vec::new(); //所有的源数据都会被提取
        
        for (name, switch) in syntax.froms.iter() {
            let res = match switch {
                &Switch::One(ref s) => self.get_table_name(s),
                &Switch::Two(ref s) => self.get_table_sub(s)
            };
            if let DResult::Table(t) = res {
                origin.push((name.to_string(), t));
            }else{
                return res;
            } 
        }
        //println!("origin={}", origin.len());
        let mut stack:Vec<i64> = Vec::new();  //暂存元组与索引。
        for _ in 0..origin.len() {stack.push(-1);}
        let mut result:Vec<HashMap<String, Data>> = Vec::new();
        let mut num: i64 = 0;
        while num >= 0 {
            if num < origin.len() as i64 {
                //处理其中一个单元。这里是递推过程。
                //println!("num={}, len={}", num, stack.len());
                let mut index = stack.get_mut(num as usize).unwrap();
                let dt = &origin.get(num as usize).unwrap().1.content();
                if *index < dt.len() as i64 -1 {
                    *index += 1;
                    num += 1;
                }else{
                    *index = -1;
                    num -= 1;
                }
            }else{
                let mut flag = true;
                //= where处理区 =========================
                let mut map = HashMap::new();//构造待用的resource。
                for (index, i) in origin.iter().enumerate() {
                    map.insert(i.0.to_string(), i.1.content()[stack[index] as usize].copy());
                }
                if syntax.wheres.li.len() > 0 {
                    let resource = &map;
                    //println!("WHERE: {}", syntax.wheres.to_string());
                    let mut que: Vec<ExpressionType> = Vec::new();
                    for exp in syntax.wheres.li.iter() {
                        match exp {
                            &ExpressionType::Var(ref prop) => {
                                let get_table_field_index = |table:&str, field:&str | {
                                    for &(ref s, ref d) in origin.iter() {
                                        if s == table {
                                            for (i, d) in d.head.iter().enumerate() {
                                                if d == field {return i as i64;}
                                            }
                                        }
                                    }
                                    return -1;
                                };
                                if prop.len() == 1 {
                                    if resource.len() > 1 {
                                        return DResult::String(format!("Syntax error: please give a name for table when there are more tables."));
                                    }else if resource.len() == 0{
                                        return DResult::String(format!("Syntax error: no any table."));
                                    }else {
                                        let mut name = "";
                                        let mut data = &Data{li:vec![]}; //获得该tabledata的name/data list。
                                        for (k, v) in resource.iter() {name=k;data=v;}
                                        let propname = &prop[0]; // 需要取得的属性名
                                        //需要根据属性名，从origin中匹配数据列。
                                        let index = get_table_field_index(name, propname);
                                        if index >= 0 {
                                            let value = data.li[index as usize].to_expt();
                                            que.push(value);
                                        }else{
                                            return DResult::String(format!("Field {} not found.", propname));
                                        }
                                    }
                                }else if prop.len() == 0 {
                                    return DResult::String(format!("Syntax error: no prop name."));
                                }else{ //有表引用
                                    let tablename = &prop[0];
                                    let propname = &prop[1];
                                    let index = get_table_field_index(tablename, propname);
                                    if let Option::Some(data) = resource.get(tablename) {
                                        if index >= 0 {
                                            let value = data.li[index as usize].to_expt();
                                            que.push(value);
                                        }else{
                                            return DResult::String(format!("Field {} not found.", propname));
                                        }
                                    }else{
                                        return DResult::String(format!("Table {} not found.", tablename));
                                    }
                                }
                            },
                            &ExpressionType::Integer(..) |
                            &ExpressionType::Float(..) |
                            &ExpressionType::Str(..) |
                            &ExpressionType::Bool(..) => {
                                que.push(exp.copy());
                            },
                            &ExpressionType::Signal(ref sign) => {
                                let pp = {
                                    let mut get_param = || { //直接从que中弹出顶端参数，并自动处理变量类型。
                                        if let Option::Some(some) = que.pop(){
                                            return match some { //expt
                                                ExpressionType::Integer(..) |
                                                ExpressionType::Float(..) |
                                                ExpressionType::Str(..) |
                                                ExpressionType::Bool(..) => Result::Ok(some.copy()),
                                                ExpressionType::Var(_) => {Result::Err(format!("invalid var."))},
                                                _ => {Result::Err(format!("invalid type."))}
                                            };
                                        }else{
                                            Result::Err(format!("Syntax expression error."))
                                        }
                                    };
                                    match sign.as_str() {
                                        "^" | "*" | "/" | "+" | "-" | "%" |
                                        ">" | "<" | ">="  |"<=" | "=" | "!=" |
                                        "&&" | "||" => {
                                            let p2 = match get_param() {
                                                Result::Ok(ok) => ok,
                                                Result::Err(e) => {return DResult::String(e);}
                                            };
                                            let p1 = match get_param() {
                                                Result::Ok(ok) => ok,
                                                Result::Err(e) => {return DResult::String(e);}
                                            };
                                            match ExpressionType::make_two(&p1, &p2, sign.as_str()) {
                                                Result::Ok(ok) => ok,
                                                Result::Err(e) => {return DResult::String(e);}
                                            }
                                            
                                        },
                                        "!" => {
                                            let p1 = match get_param() {
                                                Result::Ok(ok) => ok,
                                                Result::Err(e) => {return DResult::String(e);}
                                            };
                                            match ExpressionType::make_one(&p1, sign.as_str()) {
                                                Result::Ok(ok) => ok,
                                                Result::Err(e) => {return DResult::String(e);}
                                            }
                                            
                                        },
                                        _ => {
                                            return DResult::String(format!("Unknown operator."));
                                        }
                                    }
                                };
                                que.push(pp);
                            },
                            _ => {}
                        }
                    }
                    if que.len() == 1 {
                        if let &ExpressionType::Bool(b) = &que[0] {
                            flag = b;
                        }else {
                            return DResult::String(format!("Wrong where expression."));
                        }
                    }else {
                        return DResult::String(format!("Wrong where expression."));
                    }
                }

                //= where处理结束 =======================
                if flag { //flag
                    result.push(map);
                }
                num -= 1;
            }
        }
        //ORDER
        //预处理orders列表。
        let mut ord:Vec<(usize, String, usize, String, bool)> = Vec::new();
        for i in syntax.orders.iter() {
            //在origin中的索引，表名，列索引，列名，desc
            if let Option::Some(u) = i.0.find(".") {
                let tablename = i.0[..u].to_string();
                let columnname = i.0[u+1..].to_string();
                let mut tableindex = -1;
                for (idx, tup) in origin.iter().enumerate() {
                    if tup.0 == tablename {
                        tableindex = idx as i32;
                        break;
                    }
                }
                if tableindex >= 0 {  //找到了对应的表名
                    let mut flag = true;
                    for (idx, d) in origin[tableindex as usize].1.head.iter().enumerate() {  //搜索对应的列名
                        if d.to_string() == columnname {
                            ord.push((tableindex as usize, tablename.to_string(), idx, columnname.to_string(), i.1));
                            flag = false;
                            break;
                        }
                    }
                    if flag {
                        return DResult::String(format!("Column {} is not found in {}.", columnname, tablename));
                    }
                }else{
                    return DResult::String(format!("Table {} is not found.", tablename));
                }
            }else{  //这表示没有多表
                if origin.len() == 1 {
                    let mut tablename = String::new();
                    let columnname = i.0.to_string();
                    let tableindex = 0;
                    let mut columnindex = -1;
                    for tup in origin.iter() {
                        tablename = tup.0.to_string();
                        for (idx, d) in tup.1.head.iter().enumerate() {
                            if d.to_string() == columnname {
                                columnindex = idx as i32;
                                break;
                            }
                        }
                    }
                    if columnindex >= 0 {
                        ord.push((tableindex, tablename.to_string(), columnindex as usize, columnname.to_string(), i.1));
                    }else{
                        return DResult::String(format!("Column {} is not found.", columnname));
                    }
                }else{
                    return DResult::String(format!("You need give a name for order column because there are many origins."));
                }
            }
        }
        result.sort_by(|a, b|{
            //排序时获得的a/b是HashMap<String, Data>.对照origin中的数据列名，搜索ord列表中的每一个项并优先级判断。
            for &(_, ref tname, cindex, _, desc) in ord.iter() {
                //找到a/b的[tname]的[cindex]的dataitem的值，然后做比较。
                //二者相等时，将ord推向下一个loop;二者不相等时，根据desc返回less/great。
                //如果二者不能比较，按照相等做判定。
                let p1 = &a[tname].li[cindex];
                let p2 = &b[tname].li[cindex];
                let mut result = if let Result::Ok(ok) = p1.cmp(p2){ok}else{Ordering::Equal};
                if !desc {
                    if result == Ordering::Less {result = Ordering::Greater;}
                    else if result == Ordering::Greater {result = Ordering::Less;}
                }
                if result != Ordering::Equal {
                    return result;
                }
            }
            return Ordering::Equal;
        });
        if false { //result的输出测试
            for (i, j) in result.iter().enumerate() {
                println!("{}:", i);
                for (k, v) in j.iter() {
                    println!("    {}: {}", k.to_string(), v.to_string());
                }
            }
        }
        //构造目标列
        //分两种情况。如果goal的第一个是"*"，将其判定为全部字段；否则按照goals来。
        let mut head = vec![];
        let mut all_head = false;
        let mut self_construct_goals = vec![];
        let mut ref_goals = &syntax.goals;
        if syntax.goals.len() == 1 {
            let li = &syntax.goals.get(0).unwrap().1.li;
            if li.len() == 1  {
                if let ExpressionType::Signal(ref s) = li[0] {
                    if s == "*" {
                        //确认判定为all.
                        all_head = true;
                        
                        //如果只有1个origin，不加表名，否则加表名。
                        if origin.len() == 1 {
                            let dt = &origin[0].1;
                            for i in dt.head.iter() {
                                head.push(i.to_string());
                                self_construct_goals.push((i.to_string(), Expression::new_single(i)));
                            }
                        }else{
                            for &(ref dtname, ref dt) in origin.iter() {
                                for i in dt.head.iter() {
                                    let title = format!("{}.{}", dtname, i);
                                    head.push(title.to_string());
                                    self_construct_goals.push((title.to_string(), Expression{li: vec![ExpressionType::Var(vec![
                                        dtname.to_string(), i.to_string()
                                    ])], setence: format!("{}.{}", dtname, i)}));
                                }
                            }
                        }
                        ref_goals = &mut self_construct_goals;
                    }
                }
            }
        }
        if !all_head {
            for &(ref name, _) in syntax.goals.iter() {
                head.push(name.to_string());
            }
        }
        let mut content:Vec<Data> = vec![];
        for resource in result.iter() { //这个是遍历数据行，然后输出content。
            let mut content_sub = vec![];
            for &(_, ref expression) in ref_goals.iter() { //这个是遍历列目标。
                // 执行expression
                let mut que:Vec<ExpressionType> = vec![]; //中转存储区。
                
                for exp in expression.li.iter() {
                    match exp {
                        &ExpressionType::Var(ref prop) => {
                            let get_table_field_index = |table:&str, field:&str| {
                                for &(ref s, ref d) in origin.iter() {
                                    if s == table {
                                        for (i, d) in d.head.iter().enumerate() {
                                            if d == field {return i as i64;}
                                        }
                                    }
                                }
                                return -1;
                            };
                            if prop.len() == 1 {
                                if resource.len() > 1 {
                                    return DResult::String(format!("Syntax error: please give a name for table when there are more tables."));
                                }else if resource.len() == 0{
                                    return DResult::String(format!("Syntax error: no any table."));
                                }else {
                                    let mut name = "";
                                    let mut data = &Data{li:vec![]}; //获得该tabledata的name/data list。
                                    for (k, v) in resource.iter() {name=k;data=v;}
                                    let propname = &prop[0]; // 需要取得的属性名
                                    //需要根据属性名，从origin中匹配数据列。
                                    let index = get_table_field_index(name, propname);
                                    if index >= 0 {
                                        let value = data.li[index as usize].to_expt();
                                        que.push(value);
                                    }else{
                                        return DResult::String(format!("Field {} not found.", propname));
                                    }
                                }
                            }else if prop.len() == 0 {
                                return DResult::String(format!("Syntax error: no prop name."));
                            }else{ //有表引用
                                let tablename = &prop[0];
                                let propname = &prop[1];
                                let index = get_table_field_index(tablename, propname);
                                if let Option::Some(data) = resource.get(tablename) {
                                    if index >= 0 {
                                        let value = data.li[index as usize].to_expt();
                                        que.push(value);
                                    }else{
                                        return DResult::String(format!("Field {} not found.", propname));
                                    }
                                }else{
                                    return DResult::String(format!("Table {} not found.", tablename));
                                }
                            }
                        },
                        &ExpressionType::Integer(..) |
                        &ExpressionType::Float(..) |
                        &ExpressionType::Str(..) |
                        &ExpressionType::Bool(..) => {
                            que.push(exp.copy());
                        },
                        &ExpressionType::Signal(ref sign) => {
                            let pp = {
                                let mut get_param = || { //直接从que中弹出顶端参数，并自动处理变量类型。
                                    if let Option::Some(some) = que.pop(){
                                        return match some { //expt
                                            ExpressionType::Integer(..) |
                                            ExpressionType::Float(..) |
                                            ExpressionType::Str(..) |
                                            ExpressionType::Bool(..) => Result::Ok(some.copy()),
                                            ExpressionType::Var(_) => {Result::Err(format!("invalid var."))},
                                            _ => {Result::Err(format!("invalid type."))}
                                        };
                                    }else{
                                        Result::Err(format!("Syntax expression error."))
                                    }
                                };
                                match sign.as_str() {
                                    "^" | "*" | "/" | "+" | "-" | "%" |
                                    ">" | "<" | ">="  |"<=" | "=" | "!=" |
                                    "&&" | "||" => {
                                        let p2 = match get_param() {
                                            Result::Ok(ok) => ok,
                                            Result::Err(e) => {return DResult::String(e);}
                                        };
                                        let p1 = match get_param() {
                                            Result::Ok(ok) => ok,
                                            Result::Err(e) => {return DResult::String(e);}
                                        };
                                        match ExpressionType::make_two(&p1, &p2, sign.as_str()) {
                                                Result::Ok(ok) => ok,
                                                Result::Err(e) => {return DResult::String(e);}
                                            }
                                        
                                    },
                                    "!" => {
                                        let p1 = match get_param() {
                                            Result::Ok(ok) => ok,
                                            Result::Err(e) => {return DResult::String(e);}
                                        };
                                        match ExpressionType::make_one(&p1, sign.as_str()) {
                                                Result::Ok(ok) => ok,
                                                Result::Err(e) => {return DResult::String(e);}
                                            }
                                        
                                    },
                                    _ => {
                                        return DResult::String(format!("Unknown operator."));
                                    }
                                }
                            };
                            que.push(pp);
                        },
                        _ => {}
                    }
                }
                //最后剩下的一个是运算结果。
                if que.len() == 1 {
                    //println!("final result is {}", que[0].to_string());
                    content_sub.push(DataItem::from_expt(&que[0]));
                }else {
                    return DResult::String(format!("Wrong expression."));
                }
                //end expression
            }
            content.push(Data{li: content_sub});
        }
        if syntax.distinct {
            //去重
            //暴力解决问题。
            let mut i = 0;
            let mut len = content.len();
            while i < len {
                let mut j = i + 1;
                while j < len {
                    if content[i].eq(&content[j]) {
                        content.remove(j);
                        len -= 1;
                    }else{
                        j += 1;
                    }
                }
                i += 1;
            }
        }
        DResult::Table(DTable{
            head: head,
            content: content
        })
    }
    fn get_table_name(&mut self, table_name:&str) -> DResult {
        //这个函数返回一份可迭代的数据表格。
        if ! self.conf.tables.contains_key(table_name) {
            if self.conf.views.contains_key(table_name) {
                return self.get_view_name(table_name);
            }else {
                return DResult::String(format!("Table {} is not exists.", table_name));
            }
        }
        let mut records = vec![];
        let table_conf = &self.conf.tables[table_name]; // 获得该表格的配置信息。

        let mut head = vec![];
        for i in table_conf.fields.iter() {
            head.push(i.name.to_string());
        }
        let mut temp = table_conf.get_template(); // 获得数据模板
        if ! self.conf.table_pages.contains_key(table_name) {
            self.conf.table_pages.insert(table_name.to_string(), vec![]);
        }
        let pages = &self.conf.table_pages[table_name][..];
        for i in 0..table_conf.count {
            self.file.read(pages, self.conf.pages.len(), i, &mut temp);
            records.push(temp.copy());
        }
        DResult::Table(DTable{
            head: head,
            content: records
        })
    }
    fn get_view_name(&mut self, view_name:&str) -> DResult {
        //这个函数返回一个视图的结果。
        if !self.conf.views.contains_key(view_name) {
            return DResult::String(format!("View {} is not exists.", view_name)); 
        }
        let view = self.conf.views[view_name].copy();
        self.get_table_sub(&view)
    }
    pub fn select(&mut self, syntax:&SelectSyntax) -> DResult {
        self.get_table_sub(syntax)
    }
    pub fn insert_into(&mut self, syntax:&InsertSyntax) -> DResult {
        //插入一组数据到表格中。它们会被追加到末尾。
        //插入操作可能扩展新页，因此需要根据返回结果更改页记录。
        if ! self.conf.tables.contains_key(syntax.table_name.as_str()) {
            return DResult::String(format!("Table is not exists."));
        }
        let mut table_conf = self.conf.tables.remove(syntax.table_name.as_str()).unwrap();
        //获取页号列表
        if ! self.conf.table_pages.contains_key(syntax.table_name.as_str()) {
            self.conf.table_pages.insert(syntax.table_name.to_string(), vec![]);
        }
        let mut pages = self.conf.table_pages.remove(syntax.table_name.as_str()).unwrap();
        let mut count = 0;
        let mut result = DResult::None;
        'outer: for i in syntax.values.iter() {  // 遍历数据行
            let mut li = Vec::new();
            let mut enable_index = 0;  //记录某一个列的有效索引。指排除autoinc之外的列的索引。
            for j in table_conf.fields.iter() {  // 遍历该表的字段列表，按顺序添加数据
                if !syntax.has_head { //存在表头标记
                    if j.auto_inc && j.t == FieldType::Integer {
                        if ! table_conf.auto_config.contains_key(j.name.as_str()) {
                            table_conf.auto_config.insert(j.name.to_string(), 1);
                        }
                        {
                            let v = table_conf.auto_config.get_mut(j.name.as_str()).unwrap();
                            let dataitem = DataItem::Integer(*v as i64);
                            *v += 1;
                            li.push(dataitem);
                        }
                    }else if i.contains_key(enable_index.to_string().as_str()){
                        let mut dataitem = j.t.get_dataitem();
                        DataItem::from_dfa(&i[enable_index.to_string().as_str()], &mut dataitem);
                        li.push(dataitem);
                        enable_index += 1;
                    }else {
                        result = DResult::String(format!("No match value for field {}.", j.name));
                        break 'outer;
                    }
                }else{
                    if j.auto_inc && j.t == FieldType::Integer {
                        //print!("AUTO_INC");
                        if ! table_conf.auto_config.contains_key(j.name.as_str()) {
                            table_conf.auto_config.insert(j.name.to_string(), 1);
                        }
                        {let v = table_conf.auto_config.get_mut(j.name.as_str()).unwrap();
                        //println!("value={}", *v);
                        let dataitem = DataItem::Integer(*v as i64);
                        *v += 1;
                        li.push(dataitem);}
                        //println!("auto_inc: value={}", table_conf.auto_config.get(j.name.as_str()).unwrap());
                    }else if i.contains_key(j.name.as_str()) {  // 存在该数据
                        let mut dataitem = j.t.get_dataitem();
                        DataItem::from_dfa(&i[j.name.as_str()], &mut dataitem);
                        li.push(dataitem);
                    }else if let Option::Some(ref value) = j.default {
                        let dataitem = value.copy();
                        li.push(dataitem);
                    }else{
                        result = DResult::String(format!("Error: field {} has no default value and cannot find its value.", j.name.as_str()));
                        break 'outer;
                    }
                }
            }
            
            //查重与确认操作。主要查：
            /*
                1. primary重复
                2. unique重复
                3. 外键约束
            */
            let success = true;
            let mut temp = table_conf.get_template();
            for i in 0..table_conf.count {
                self.file.read(&pages[..], self.conf.pages.len(), i, &mut temp);
                let mut now_success = true;
                //primary查重
                let mut primary_flag = true;
                for (index, p) in table_conf.fields.iter().enumerate() {
                    if primary_flag && p.primary && !temp.li[index].eq(&li[index]) {primary_flag = false;}
                    if p.unique && temp.li[index].eq(&li[index]) {now_success = false;break;}
                }
                if !now_success{
                    result = DResult::String(format!("Primary constriant is not satisfied."));
                    break 'outer;
                }
            }
            for (index, p) in table_conf.fields.iter().enumerate() {
                if table_conf.foreign.contains_key(p.name.as_str()) {
                    //存在外键约束。需要检查外表的存在与数据的存在。
                    let item = &li[index];
                    let foreign = table_conf.foreign.get(p.name.as_str()).unwrap();
                    if let DResult::Table(ref dt) = self.get_table_name(foreign.foreign_table.as_str()) {
                        //已经提取到了该表的所有数据。直接提取列并且检查。
                        let mut now_success = false;
                        for c in dt.get_column(foreign.foreign_field.as_str()) {
                            if c.eq(item) {
                                now_success = true;
                                break;
                            }
                        }
                        if !now_success {
                            result = DResult::String(format!("Foreign constriant is failed."));
                            break 'outer;
                        }
                    }else{
                        result = DResult::String(format!("Foreign constriant is failed."));
                        break 'outer;
                    }
                }
            }
            //准备完成，开始写文件
            if success {
                let data = Data::new(li);
                if let Option::Some(u) = self.file.write(&pages[..], self.conf.pages.len(), table_conf.count, &data) {
                    //u表示最新的页号。从page.len()->u的所有页号都是新的页号。
                    for i in self.conf.pages.len()..u+1 {
                        self.conf.pages.push(PageType::Data(syntax.table_name.to_string()));
                        pages.push(i);
                    }
                }
                count += 1;
                table_conf.count += 1;
            }
        }
        //为了不违反rust的mut借用规则，这个地方只能这么写，先把内容提取出来在最后再插入回去。
        self.conf.tables.insert(syntax.table_name.to_string(), table_conf);
        self.conf.table_pages.insert(syntax.table_name.to_string(), pages);
        if let DResult::None = result {
            DResult::String(format!("{} record(s) has inserted.", count))
        }else {
            result
        }
        
    }
    pub fn update(&mut self, syntax:&UpdateSyntax) -> DResult {
        //update的语序：
        /*  1. 逐个读取表中的所有记录
            2. 对当前记录执行where语句，判断是否符合条件
            3. 如果符合条件就覆写当前记录
        */
        let table_name = syntax.table_name.as_str();
        if ! self.conf.tables.contains_key(table_name) {
            return DResult::String(format!("Table {} is not exists.", table_name));
        }
        let table_conf = &self.conf.tables[table_name]; // 获得该表格的配置信息。

        let mut head = vec![];  // 获得表格的head。
        for i in table_conf.fields.iter() {head.push(i.name.to_string());}
        //for i in head.iter() {print!("[{}]", i);}

        //获得set的覆盖模板。
        let mut set_temp = Vec::new();
        for (index, i) in head.iter().enumerate() {
            if syntax.sets.contains_key(i) {
                let v = syntax.sets.get(i).unwrap();
                set_temp.push(Option::Some(match table_conf.fields[index].t{
                    FieldType::Integer => if let &DfaWord::Integer(i) = v {DataItem::Integer(i)}
                        else{return DResult::String(format!("Wrong where expression type."));},
                    FieldType::Float => if let &DfaWord::Float(f) = v {DataItem::Float(f)}
                        else{return DResult::String(format!("Wrong where expression type."));},
                    FieldType::Bool => if let &DfaWord::Bool(b) = v {DataItem::Bool(b)}
                        else{return DResult::String(format!("Wrong where expression type."));},
                    FieldType::Str(u) => if let &DfaWord::Str(ref s) = v {DataItem::Str(u, s.to_string())}
                        else{return DResult::String(format!("Wrong where expression type."));}
                }));
            }else{
                set_temp.push(Option::None);
            }
        }

        let mut temp = table_conf.get_template(); // 获得数据模板
        if ! self.conf.table_pages.contains_key(table_name) {
            self.conf.table_pages.insert(table_name.to_string(), vec![]);
        }
        let pages = &self.conf.table_pages[table_name][..];
        let mut count = 0;
        for i in 0..table_conf.count {
            self.file.read(pages, self.conf.pages.len(), i, &mut temp);
            // 执行第2步，开始判断。
            let mut flag = true;
            if syntax.wheres.li.len() > 0 {
                let resource = &temp;
                let mut que: Vec<ExpressionType> = Vec::new();
                for exp in syntax.wheres.li.iter() {
                    match exp {
                        &ExpressionType::Var(ref prop) => {
                            let get_field_index = |field:&str| {
                                for (i, s) in head.iter().enumerate() {
                                    if s.to_string() == field.to_string() {
                                        return i as i64;
                                    }
                                }
                                return -1;
                            };
                            if prop.len() == 1 {
                                let propname = &prop[0];
                                //println!("propname: {}", propname);
                                let index:i64 = get_field_index(propname);
                                if index >= 0 {
                                    let value = resource.li[index as usize].to_expt();
                                    que.push(value);
                                }else{
                                    return DResult::String(format!("Field {} not found.", propname));
                                }
                            }else{
                                return DResult::String(format!("Syntax error: update syntax donot allow mutli tables."));
                            }
                        },
                        &ExpressionType::Integer(..) |
                        &ExpressionType::Float(..) |
                        &ExpressionType::Str(..) |
                        &ExpressionType::Bool(..) => {
                            que.push(exp.copy());
                        },
                        &ExpressionType::Signal(ref sign) => {
                            let pp = {
                                let mut get_param = || { //直接从que中弹出顶端参数，并自动处理变量类型。
                                    if let Option::Some(some) = que.pop(){
                                        return match some { //expt
                                            ExpressionType::Integer(..) |
                                            ExpressionType::Float(..) |
                                            ExpressionType::Str(..) |
                                            ExpressionType::Bool(..) => Result::Ok(some.copy()),
                                            ExpressionType::Var(_) => {Result::Err(format!("invalid var."))},
                                            _ => {Result::Err(format!("invalid type."))}
                                        };
                                    }else{
                                        Result::Err(format!("Syntax expression error."))
                                    }
                                };
                                match sign.as_str() {
                                    "^" | "*" | "/" | "+" | "-" | "%" |
                                    ">" | "<" | ">="  |"<=" | "=" | "!=" |
                                    "&&" | "||" => {
                                        let p2 = match get_param() {
                                            Result::Ok(ok) => ok,
                                            Result::Err(e) => {return DResult::String(e);}
                                        };
                                        let p1 = match get_param() {
                                            Result::Ok(ok) => ok,
                                            Result::Err(e) => {return DResult::String(e);}
                                        };
                                        match ExpressionType::make_two(&p1, &p2, sign.as_str()) {
                                                Result::Ok(ok) => ok,
                                                Result::Err(e) => {return DResult::String(e);}
                                            }
                                        
                                    },
                                    "!" => {
                                        let p1 = match get_param() {
                                            Result::Ok(ok) => ok,
                                            Result::Err(e) => {return DResult::String(e);}
                                        };
                                        match ExpressionType::make_one(&p1, sign.as_str()) {
                                                Result::Ok(ok) => ok,
                                                Result::Err(e) => {return DResult::String(e);}
                                            }
                                        
                                    },
                                    _ => {
                                        return DResult::String(format!("Unknown operator."));
                                    }
                                }
                            };
                            que.push(pp);
                        },
                        _ => {}
                    }
                }
                if que.len() == 1 {
                    if let &ExpressionType::Bool(b) = &que[0] {
                        flag = b;
                    }else {
                        return DResult::String(format!("Wrong where expression."));
                    }
                }else {
                    return DResult::String(format!("Wrong where expression."));
                }
            }
            if flag { //执行3，确认修改本记录。
                count += 1;
                for (i, m) in set_temp.iter().enumerate() {
                    if let &Option::Some(ref some) = m {
                        temp.li[i] = some.copy();
                    }
                }
                self.file.write(pages, self.conf.pages.len(), i, &temp);
            }
        }
        DResult::String(format!("{} record(s) updated.", count))
    }
    pub fn delete(&mut self, syntax:&DeleteSyntax) -> DResult {
        //delete的语序：
        /*  1. 逐条读取record中的所有记录
            2. 判断某一条记录是否应该被删除。如果需要，记下其seek
            3. 从表格的后方依次往前读，读出n条不需要被删除的数据前移。要前移的数据seek必须大于移往的目标，防止出错。
            4. 删除每一条记录时，都要检查所有的外键引用，删掉约束对象。
        */
        let table_name = syntax.table_name.as_str();
        if ! self.conf.tables.contains_key(table_name) {
            return DResult::String(format!("Table {} is not exists.", table_name));
        }
        let table_conf = self.conf.tables.get_mut(table_name).unwrap(); // 获得该表格的配置信息。

        let mut head = vec![];  // 获得表格的head。
        for i in table_conf.fields.iter() {head.push(i.name.to_string());}

        let mut temp = table_conf.get_template(); // 获得数据模板
        if ! self.conf.table_pages.contains_key(table_name) {
            self.conf.table_pages.insert(table_name.to_string(), vec![]);
        }
        let pages = &self.conf.table_pages[table_name][..];
        let mut seeks = Vec::new(); //需要删除的标记列表。
        let mut seeks_set = HashSet::new();
        for i in 0..table_conf.count {
            self.file.read(pages, self.conf.pages.len(), i, &mut temp);
            // 执行第2步，开始判断。
            let mut flag = true;
            if syntax.wheres.li.len() > 0 {
                let resource = &temp;
                let mut que: Vec<ExpressionType> = Vec::new();
                for exp in syntax.wheres.li.iter() {
                    match exp {
                        &ExpressionType::Var(ref prop) => {
                            let get_field_index = |field:&str| {
                                for (i, s) in head.iter().enumerate() {
                                    if s.to_string() == field.to_string() {
                                        return i as i64;
                                    }
                                }
                                return -1;
                            };
                            if prop.len() == 1 {
                                let propname = &prop[0];
                                //println!("propname: {}", propname);
                                let index:i64 = get_field_index(propname);
                                if index >= 0 {
                                    let value = resource.li[index as usize].to_expt();
                                    que.push(value);
                                }else{
                                    return DResult::String(format!("Field {} not found.", propname));
                                }
                            }else{
                                return DResult::String(format!("Syntax error: update syntax donot allow mutli tables."));
                            }
                        },
                        &ExpressionType::Integer(..) |
                        &ExpressionType::Float(..) |
                        &ExpressionType::Str(..) |
                        &ExpressionType::Bool(..) => {
                            que.push(exp.copy());
                        },
                        &ExpressionType::Signal(ref sign) => {
                            let pp = {
                                let mut get_param = || { //直接从que中弹出顶端参数，并自动处理变量类型。
                                    if let Option::Some(some) = que.pop(){
                                        return match some { //expt
                                            ExpressionType::Integer(..) |
                                            ExpressionType::Float(..) |
                                            ExpressionType::Str(..) |
                                            ExpressionType::Bool(..) => Result::Ok(some.copy()),
                                            ExpressionType::Var(_) => {Result::Err(format!("invalid var."))},
                                            _ => {Result::Err(format!("invalid type."))}
                                        };
                                    }else{
                                        Result::Err(format!("Syntax expression error."))
                                    }
                                };
                                match sign.as_str() {
                                    "^" | "*" | "/" | "+" | "-" | "%" |
                                    ">" | "<" | ">="  |"<=" | "=" | "!=" |
                                    "&&" | "||" => {
                                        let p2 = match get_param() {
                                            Result::Ok(ok) => ok,
                                            Result::Err(e) => {return DResult::String(e);}
                                        };
                                        let p1 = match get_param() {
                                            Result::Ok(ok) => ok,
                                            Result::Err(e) => {return DResult::String(e);}
                                        };
                                        match ExpressionType::make_two(&p1, &p2, sign.as_str()) {
                                                Result::Ok(ok) => ok,
                                                Result::Err(e) => {return DResult::String(e);}
                                            }
                                        
                                    },
                                    "!" => {
                                        let p1 = match get_param() {
                                            Result::Ok(ok) => ok,
                                            Result::Err(e) => {return DResult::String(e);}
                                        };
                                        match ExpressionType::make_one(&p1, sign.as_str()) {
                                                Result::Ok(ok) => ok,
                                                Result::Err(e) => {return DResult::String(e);}
                                            }
                                        
                                    },
                                    _ => {
                                        return DResult::String(format!("Unknown operator."));
                                    }
                                }
                            };
                            que.push(pp);
                        },
                        _ => {}
                    }
                }
                if que.len() == 1 {
                    if let &ExpressionType::Bool(b) = &que[0] {
                        flag = b;
                    }else {
                        return DResult::String(format!("Wrong where expression."));
                    }
                }else {
                    return DResult::String(format!("Wrong where expression."));
                }
            }
            if flag { //执行3，确认删除改本记录。
                seeks.push(i);
                seeks_set.insert(i);
            }
        }
        //print!("DELETE:[");
        //for i in seeks.iter() {print!("{},", i);}
        //println!("]");
        //执行批量删除＋前移
        let mut last = table_conf.count as i64;
        for i in seeks.iter() {
            //首先确定下一个需要前移的数据源。
            let mut flag = false;
            //print!("[{}]", i);
            loop {
                //print!("[last:{}]", last);
                last -= 1;
                if last < 0 {flag = true;break;}
                else if last <= *i as i64 {break;}
                else if !seeks_set.contains(&(last as usize)) {
                    //这表示last可用，将last移到i上。
                    //println!("DELETE: FROM {} TO {}.", last, *i);
                    self.file.read(pages, self.conf.pages.len(), last as usize, &mut temp);
                    self.file.write(pages, self.conf.pages.len(), *i, &temp);
                    break;
                }
            }
            if flag {break;}
            seeks_set.remove(i);
        }
        table_conf.count -= seeks.len();
        DResult::String(format!("{} record(s) deleted.", seeks.len()))
    }
    pub fn create_table(&mut self, syntax:&CreateTableSyntax) -> DResult {
        // 根据syntax直接映射表conf.
        if self.conf.tables.contains_key(&syntax.name) {
            return DResult::String(format!("Table is already exists."));
        }
        if syntax.name.trim() == "".to_string() {
            return DResult::String(format!("Table name cannot be empty."));
        }
        let mut fields = Vec::with_capacity(syntax.fields.len());
        let mut primary = vec![];
        for i in syntax.fields.iter() {
            let tp = match i.t.as_str() {
                "integer" => FieldType::Integer,
                "float" => FieldType::Float,
                "bool" => FieldType::Bool,
                _ => {
                    if i.t.starts_with("str:") {
                        FieldType::Str(i.t[4..].parse().unwrap())
                    }else{
                        panic!("Wrong type.");
                    }
                }
            };
            fields.push(FieldConfig{
                name: i.name.to_string(),
                unique: i.unique,
                primary: i.primary,
                not_null: i.not_null,
                auto_inc: i.auto_inc,
                default: match i.default {
                    Option::None => Option::None,
                    Option::Some(ref s) => Option::Some(match tp {
                        FieldType::Integer => DataItem::Integer(s.parse().unwrap()),
                        FieldType::Float => DataItem::Float(s.parse().unwrap()),
                        FieldType::Bool => DataItem::Bool(s.parse().unwrap()),
                        FieldType::Str(u) => DataItem::Str(u, s.to_string())
                    })
                },
                t: tp,
            });
            if i.primary {primary.push(i.name.to_string());}
        }
        let mut foreign = HashMap::new();
        for i in syntax.foreigns.iter() {
            foreign.insert(i.field.to_string(), ForeignConfig{
                field: i.field.to_string(),
                foreign_table: i.foreign_table.to_string(),
                foreign_field: i.foreign_field.to_string(),
                delete_action: match i.delete_action.as_str() {
                    "cascade" => ForeignType::Cascade,
                    "restrict" => ForeignType::Restrict,
                    "setnull" => ForeignType::SetNull,
                    _ => ForeignType::Cascade
                }
            });
        }
        let conf = TableConfig {
            name: syntax.name.to_string(),
            fields: fields,
            primary: primary,
            auto_config: HashMap::new(),
            foreign: foreign,
            count: 0
        };
        let ret = DResult::String(format!("Table {} has created.", conf.name.to_string()));
        self.conf.tables.insert(conf.name.to_string(), conf);
        ret
    }
    pub fn alter_table(&mut self, syntax:&AlterTableSyntax) -> DResult {
        //alter的语序：
        /*  1. 首先进行全盘检查。
            2. 检查add部分，是否存在重名字段（如果字段在drop列表内则不算，这会算作d/a）
            3. 检查add部分的default值。如果该表数据不为0，那么必须存在default值。
            3. 检查alter部分，添加unique/primary属性需要做约束检查。
        */
        let table_name = syntax.name.as_str();
        if ! self.conf.tables.contains_key(table_name) {
            return DResult::String(format!("Table {} is not exists.", table_name));
        }
        let mut table_conf = self.conf.tables.get_mut(table_name).unwrap(); // 获得该表格的配置信息。

        let mut head = vec![];  // 获得表格的head。
        for i in table_conf.fields.iter() {head.push(i.name.to_string());}

        let mut temp = table_conf.get_template(); // 获得数据模板
        if ! self.conf.table_pages.contains_key(table_name) {
            self.conf.table_pages.insert(table_name.to_string(), vec![]);
        }
        let pages = self.conf.table_pages.get_mut(table_name).unwrap();
        //首先抓取全部的数据
        let mut old_list = Vec::new();
        for index in 0..table_conf.count {
            self.file.read(&pages[..], self.conf.pages.len(), index, &mut temp);
            old_list.push(temp.copy());
        }
        //然后开始检查syntax.
        //add
        for f in syntax.adds.iter() {
            //add的字段名不能与现有名重复。除非该字段名在drop列表内。
            if has(&table_conf.fields, |i|i.name == f.name) {
                if ! has(&syntax.drops, |i|i.to_string() == f.name) {
                    return DResult::String(format!("Add field {} is repeated.", f.name));
                }
            }
            //检查add字段的default值。只要count>0，default必须不为空。
            //检查add字段的unique值。只要count>0，就不允许新的unique。
            //检查add字段的primary值。只要count>0，就不允许新的primary。
            if table_conf.count > 0 {
                if let Option::None = f.default {
                    return DResult::String(format!("Add field {} must have a default value.", f.name));
                }
                if f.unique {
                    return DResult::String(format!("Add field {} cannot be unique.", f.name));
                }
                if f.primary {
                    return DResult::String(format!("Add field {} cannot be primary key.", f.name));
                }
            }
        }
        //alter
        for f in syntax.alters.iter() {
            //alter字段名必须存在。
            let mut index = 0;
            for i in head.iter() {
                if i.as_str() == f.name.as_str() {break;}//同时抓取该属性的下标。
                index += 1;
            }
            if index >= head.len() {
                return DResult::String(format!("Alter field {} is not exists.", f.name));
            }
            //检查unique值。现有数据如果存在非unqiue值，就拒绝unique约束。
            if f.unique && !table_conf.fields[index].unique && table_conf.count > 0 {
                //直接用粗暴的检查方法。
                for i in 0..old_list.len() {
                    for j in i+1..old_list.len() {
                        if old_list[i].li[index].eq(&old_list[j].li[index]) {
                            return DResult::String(format!("Alter field {} cannot be unique: repeat value in records.", f.name));
                        }
                    }
                }
            }
            //检查primary值。不允许新的primary约束。
            if f.primary {
                return DResult::String(format!("Alter field {} cannot be primary key.", f.name));
            }
            //检查type。如果type不同，就要求新的default。
            if f.t != table_conf.fields[index].t.to_string() {
                if let Option::None = f.default {
                    return DResult::String(format!("Alter field {} must has a default because type is changed.", f.name));
                }
            }
        }
        //drop
        for f in syntax.drops.iter() {
            if ! has(&table_conf.fields, |i|i.name == f.to_string()) {
                return DResult::String(format!("Drop field {} is not exists.", f));
            }
            //拒绝删除主键field.
            for i in table_conf.fields.iter() {
                if f.to_string() == i.name {
                    if i.primary {
                        return DResult::String(format!("Drop field {} cannot be primary key.", f));
                    }
                }
            }
        }
        //全部检查完成之后开始修改。
        //修改方案：先将drop的删掉，然后将alter的head合并。
        //最后将add的新加入。

        //先删除drop的数据.
        let mut drop_index = Vec::new(); //获得drop的index的反向列表
        for (h_index ,h) in head.iter().enumerate() {
                if has(&syntax.drops, |i|i.as_str() == h.as_str()) {
                    drop_index.insert(0, h_index);
                }
            }
        for i in drop_index.iter() { //由于列表是反向的，这里这种删除方法不会有问题。
            head.remove(*i);
            table_conf.fields.remove(*i);
        }
        for i in &mut old_list { //删除record中的对应记录.
            for j in drop_index.iter() {
                i.li.remove(*j);
            }
        }
        //执行alter。修改表头以及数据。
        for alter in syntax.alters.iter() {
            //抓取目标alter的index.
            let mut index = 0;
            for i in table_conf.fields.iter() {
                if i.name == alter.name {break;}
                index += 1;
            }
            let default;
            if table_conf.fields[index].t.to_string() != alter.t { //类型不同时要按照default重写所有值。
                if let Option::Some(ref v) = alter.default {
                    let def = match alter.t.as_str() {
                        "integer" => DataItem::Integer(v.parse().unwrap()),
                        "float" => DataItem::Float(v.parse().unwrap()),
                        "bool" => DataItem::Bool(v.parse().unwrap()),
                        other@_ => {
                            if other.starts_with("str:") {
                                DataItem::Str(other[4..].parse().unwrap(), v.to_string())
                            }else{
                                return DResult::String(format!("Alter fields {} has a wrong type.", alter.name));
                            }
                        }
                    };
                    default = Option::Some(def.copy());
                    for i in &mut old_list {
                        i.li[index] = def.copy();
                    }
                }else{default = Option::None;}
            }else{default = Option::None;}
            //覆盖表头。
            *table_conf.fields.get_mut(index).unwrap() = FieldConfig {
                name: alter.name.to_string(),
                unique: alter.unique,
                primary: alter.primary,
                not_null: alter.not_null,
                default: default,
                auto_inc: alter.auto_inc,
                t: FieldType::from_string(alter.t.as_str())
            }
        }
        //执行add。添加表头以及默认数据。
        for add in syntax.adds.iter() {
            let default = if let Option::Some(ref v) = add.default {
                Option::Some(match add.t.as_str() {
                    "integer" => DataItem::Integer(v.parse().unwrap()),
                    "float" => DataItem::Float(v.parse().unwrap()),
                    "bool" => DataItem::Bool(v.parse().unwrap()),
                    other@_ => {
                        if other.starts_with("str:") {
                            DataItem::Str(other[4..].parse().unwrap(), v.to_string())
                        }else{
                            return DResult::String(format!("Alter fields {} has a wrong type.", add.name));
                        }
                    }
                })
            }else{Option::None};
            //插入表头。
            table_conf.fields.push(FieldConfig {
                name: add.name.to_string(),
                unique: add.unique,
                primary: add.primary,
                not_null: add.not_null,
                default: default,
                auto_inc: add.auto_inc,
                t: FieldType::from_string(add.t.as_str())
            });
            //插入数据列
            for i in &mut old_list {
                i.li.push(if let Option::Some(ref c) = table_conf.fields[table_conf.fields.len()-1].default{
                    c.copy()
                }else{panic!("Error occured: none value.")});
            }
        }
        //最后将数据回写。
        for (i, r) in old_list.iter().enumerate() {
            if let Option::Some(u) = self.file.write(&pages[..], self.conf.pages.len(), i, r) {
                for i in self.conf.pages.len()..u+1 {
                    self.conf.pages.push(PageType::Data(table_name.to_string()));
                    pages.push(i);
                }
            }
        }
        DResult::String(format!("Alter table success."))
    }
    pub fn drop_table(&mut self, syntax:&DropTableSyntax) -> DResult {
        let table_name = syntax.name.as_str();
        if ! self.conf.tables.contains_key(table_name) {
            return DResult::String(format!("Table {} is not exists.", table_name));
        }
        // 检查约束关系。
        // 如果存在任意外键链接到当前表，那么就拒绝删除
        for (name, table) in self.conf.tables.iter() {
            for (_, f) in table.foreign.iter() {
                if f.foreign_table == table_name {
                    return DResult::String(format!("Cannot delete table: foreign key constriant in {}.", name));
                }
            }
        }
        self.conf.tables.remove(table_name);
        DResult::String(format!("Table {} is deleted.", table_name))
    }
    pub fn create_view(&mut self, syntax:&CreateViewSyntax) -> DResult {
        let name = syntax.name.as_str();
        if self.conf.tables.contains_key(name) {
            return DResult::String(format!("There is a table has same name of {}.", name));
        }
        if self.conf.views.contains_key(name) {
            return DResult::String(format!("View {} is already exists.", name));
        }
        if name.trim() == "" {
            return DResult::String(format!("View name cannot be empty."));
        }
        self.conf.views.insert(name.to_string(), syntax.sub.copy());
        DResult::String(format!("View {} has created.", name))
    }
    pub fn drop_view(&mut self, syntax:&DropViewSyntax) -> DResult {
        let name = syntax.name.as_str();
        if self.conf.tables.contains_key(name) {
            return DResult::String(format!("{} is a table, not a view.", name));
        }
        if !self.conf.views.contains_key(name) {
            return DResult::String(format!("View {} is not exists.", name));
        }
        self.conf.views.remove(name);
        DResult::String(format!("View {} is deleted.", name))
    }
    pub fn help(&mut self, syntax:&HelpSyntax) -> DResult {
        // help有4种理论支持的语法。
        /*  1. database 显示所有table/view/index的信息以及对象类型
            2. table [name] 显示指定table的信息
            3. view [name] 显示指定view的信息
            4. index [name] 显示指定index的信息
        */
        match syntax.params[0].as_str() {
            "database" => {
                let mut multi = Vec::new();
                for name in self.conf.tables.keys() {
                    multi.push(MultiResult::String(format!("TABLE {}", name)));
                    match self.help_table(name) {
                        DResult::Table(ref dt) => multi.push(MultiResult::Table(dt.copy())),
                        DResult::String(ref s) => multi.push(MultiResult::String(s.to_string())),
                        DResult::Multi(ref v) => {
                            for i in v.iter() {
                                match i {
                                    &MultiResult::Table(ref dt) => multi.push(MultiResult::Table(dt.copy())),
                                    &MultiResult::String(ref s) => multi.push(MultiResult::String(s.to_string()))
                                }
                            }
                        }
                        _ => {}
                    }
                }
                for name in self.conf.views.keys() {
                    multi.push(MultiResult::String(format!("VIEW {}", name)));
                    match self.help_view(name) {
                        DResult::Table(ref dt) => multi.push(MultiResult::Table(dt.copy())),
                        DResult::String(ref s) => multi.push(MultiResult::String(s.to_string())),
                        DResult::Multi(ref v) => {
                            for i in v.iter() {
                                match i {
                                    &MultiResult::Table(ref dt) => multi.push(MultiResult::Table(dt.copy())),
                                    &MultiResult::String(ref s) => multi.push(MultiResult::String(s.to_string()))
                                }
                            }
                        }
                        _ => {}
                    }
                }
                DResult::Multi(multi)
            },
            "table" => {
                let name = syntax.params[1].as_str();
                if self.conf.tables.contains_key(name) {
                    self.help_table(name)
                }else {
                    DResult::String(format!("Table {} is not found.", name))
                }
            },
            "view" => {
                let name = syntax.params[1].as_str();
                if self.conf.views.contains_key(name) {
                    self.help_view(name)
                }else {
                    DResult::String(format!("View {} is not found.", name))
                }
            }
            other@_ => DResult::String(format!("Syntax Error: No this syntax [{}].", other))
        }
    }
    fn help_table(&self, table_name:&str) -> DResult {
        //tablename在使用之前已经经过了存在验证。
        //helptable命令会用表格罗列出table的desc信息。包括：
        //|fieldname|type|primary|unique|notnull|default|auto_inc|foreignkey| 
        let head = vec![
            "fieldname".to_string(), 
            "type".to_string(),
            "primary".to_string(),
            "unique".to_string(),
            "not_null".to_string(),
            "default".to_string(),
            "auto_inc".to_string(),
            "foreign_key".to_string()
        ];  //共8项。
        let mut content = Vec::new();
        let table_conf = self.conf.tables.get(table_name).unwrap();
        for field in table_conf.fields.iter() {
            let li = vec![
                DataItem::Str(0, field.name.to_string()),
                DataItem::Str(0, field.t.to_string()),
                DataItem::Bool(field.primary),
                DataItem::Bool(field.unique),
                DataItem::Bool(field.not_null),
                if let Option::Some(ref dt) = field.default {dt.copy()}else{DataItem::Str(0, "".to_string())},
                DataItem::Bool(field.auto_inc),
                if table_conf.foreign.contains_key(field.name.as_str()) {
                    let foreign = table_conf.foreign.get(field.name.as_str()).unwrap();
                    DataItem::Str(0, format!("{}({})", foreign.foreign_table, foreign.foreign_field))
                }else{
                    DataItem::Str(0, "".to_string())
                }
            ];
            content.push(Data{li:li});
        }
        DResult::Table(DTable{
            head: head,
            content: content
        })
    }
    fn help_view(&self, view_name:&str) -> DResult {
        //在使用之前已经经过了存在验证。
        //使用String输出视图的Select语句的定义信息。
        let result = self.conf.views[view_name].get_setence();
        DResult::String(format!("{};", result))
    }
    

    pub fn commit_config(&self) {
        //提交对配置信息的修改到文件，一般包括对表的修改。
        self.conf.save();
    }
    pub fn commit(&self) {
        //提交对数据的修改到文件。
        self.conf.save();
    }

    pub fn has_table(&self, table_name:&str) -> bool {
        self.conf.tables.contains_key(table_name)
    }
}

//= 数据库会话结果
pub enum DResult {
    None,
    String(String),
    Table(DTable),
    Multi(Vec<MultiResult>)
}
impl DResult {
    pub fn to_string(&self) -> String {
        match self {
            &DResult::None => "".to_string(),
            &DResult::String(ref s) => s.to_string(),
            &DResult::Table(ref dt) => dt.to_string(),
            &DResult::Multi(ref v) => {
                let mut s = String::new();
                for i in v.iter() {
                    s += (i.to_string() + "\n\n").as_str();
                }
                s
            }
        }
    }
    pub fn print(&self, out:&mut Write) {
        out.write(self.to_string().as_bytes()).unwrap();
    }
    pub fn printout(&self) {
        println!("{}", self.to_string());
    }
}
pub enum MultiResult {
    String(String),
    Table(DTable)
}
impl MultiResult {
    pub fn to_string(&self) -> String {
        match self {
            &MultiResult::String(ref s) => s.to_string(),
            &MultiResult::Table(ref dt) => dt.to_string()
        }
    }
}
pub struct DTable {
    pub head:Vec<String>,
    pub content:Vec<Data>
}
impl DTable {
    pub fn copy(&self) -> DTable {
        let mut head = Vec::new();
        let mut content = Vec::new();
        for i in self.head.iter() {head.push(i.to_string());}
        for i in self.content.iter() {content.push(i.copy());}
        DTable{
            head: head,
            content: content
        }
    }
    pub fn get_column(&self, s:&str) -> Vec<DataItem> {
        let mut i = 0;
        while i < self.head.len() && self.head.get(i).unwrap() != s {i+=1;}
        if i < self.head.len() {
            let mut v = Vec::new();
            for c in self.content.iter() {
                if c.li.len() > i {v.push(c.li[i].copy());}
            }
            v
        }else{vec![]}
    }
    pub fn content(&self) -> &[Data] {
        &self.content[..]
    }
    pub fn to_string(&self) -> String {
        let mut ret = "|".to_string();
        for f in self.head.iter() {
            ret += format!(" {} |", f).as_str();
        }
        ret += "\n----------------\n";
        for h in self.content.iter(){
            ret += "|";
            for l in h.li.iter() {
                ret += format!(" {} |", l.to_string()).as_str();
            }
            ret += "\n";
        }
        ret
    }
}