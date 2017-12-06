use super::ini::{Config};
use std::collections::HashMap;
use super::save::{ConfigFile, DataItem};
use super::database::{Database, DResult};
use super::super::syntax::structures::{
    Switch, Expression, ExpressionType, SelectSyntax, InsertSyntax, DeleteSyntax, UpdateSyntax,
    CreateTableSyntax, TableFieldSyntax,
    CreateDatabaseSyntax, DropDatabaseSyntax, UseSyntax, ColSyntax,
    CreateUserSyntax, AlterUserSyntax, DropUserSyntax, GrantSyntax
};
use super::super::analyse::dfa::{DfaWord};

macro_rules! hmap {
( $( $x:expr => $y:expr ),* ) => {
    {
        let mut temp_map = HashMap::new();
        $(
            temp_map.insert($x.to_string(), $y);
        )*
        temp_map
    }
};
}
// 配置数据库操作引擎

pub struct Engine{
    conf:Config,  //配置文件
    system: ConfigFile,  //系统数据库
    /*系统数据库包括如下表：
    普通数据库列表
    用户列表
    用户权限列表
    */
    pub databases: HashMap<String, ConfigFile>,  //普通数据库
}
impl Engine {
    pub fn new() -> Self {
        //加载一个数据库引擎
        //加载配置文件
        let conf = Config::load("dba.ini");
        //加载系统数据库
        let mut system = ConfigFile::new(conf.database.to_string(), conf.systembase.to_string());
        let mut databases = HashMap::new();
        //处理系统数据库内的信息
        {
            let mut system_db = system.session();
            //初始化数据库
            if !system_db.has_table("database") {
                system_db.create_table(&CreateTableSyntax{
                    name: "database".to_string(),
                    fields: vec![
                        TableFieldSyntax{name: "id".to_string(), t: "integer".to_string(), unique: true, primary: true, not_null: true, auto_inc: true, default: Option::None},
                        TableFieldSyntax{name: "name".to_string(), t: "str:64".to_string(), unique: true, primary: false, not_null: true, auto_inc: false, default: Option::None}
                    ], foreigns: vec![]
                });
                //println!("create table database.");
            }
            if !system_db.has_table("user") {
                system_db.create_table(&CreateTableSyntax{
                    name: "user".to_string(),
                    fields: vec![
                        TableFieldSyntax{name: "id".to_string(), t: "integer".to_string(), unique: true, primary: true, not_null: true, auto_inc: true, default: Option::None},
                        TableFieldSyntax{name: "username".to_string(), t: "str:24".to_string(), unique: true, primary: false, not_null: true, auto_inc: false, default: Option::None},
                        TableFieldSyntax{name: "password".to_string(), t: "str:64".to_string(), unique: false, primary: false, not_null: true, auto_inc: false, default: Option::None},
                        TableFieldSyntax{name: "is_root".to_string(), t: "bool".to_string(), unique: false, primary: false, not_null: true, auto_inc: false, default: Option::Some("false".to_string())}
                    ], foreigns: vec![]
                });
                //插入默认的超级管理员账户。
                system_db.insert_into(&InsertSyntax{
                    table_name: "user".to_string(),
                    has_head: true,
                    values: vec![hmap![
                        "username" => DfaWord::Str("root".to_string()),
                        "password" => DfaWord::Str("root".to_string()),
                        "is_root" => DfaWord::Bool(true)
                    ]]
                });
                //println!("create table database.");
            }
            if !system_db.has_table("privilege") {
                system_db.create_table(&CreateTableSyntax{
                    name: "privilege".to_string(),
                    fields: vec![
                        TableFieldSyntax{name: "id".to_string(), t: "integer".to_string(), unique: true, primary: true, not_null: true, auto_inc: true, default: Option::None},
                        TableFieldSyntax{name: "username".to_string(), t: "str:24".to_string(), unique: false, primary: false, not_null: true, auto_inc: false, default: Option::None},
                        TableFieldSyntax{name: "database".to_string(), t: "str:64".to_string(), unique: false, primary: false, not_null: true, auto_inc: false, default: Option::None},
                        TableFieldSyntax{name: "table".to_string(), t: "str:64".to_string(), unique: false, primary: false, not_null: true, auto_inc: false, default: Option::None},
                        TableFieldSyntax{name: "type".to_string(), t: "str:16".to_string(), unique: false, primary: false, not_null: true, auto_inc: false, default: Option::None}
                    ], foreigns: vec![]
                });
                //println!("create table database.");
            }
            //加载普通数据库配置,从系统数据库读取数据库列表，然后依次加载配置文件。
            if let DResult::Table(ref table) = system_db.select(&SelectSyntax{
                distinct: false,
                froms: hmap!["database" => Switch::One("database".to_string())],
                goals: vec![("name".to_string(), Expression::new_single("name"))],
                wheres: Expression::empty(),
                orders: vec![]
            }) {
                //println!("select count: {}", table.content.len());
                for i in table.get_column("name").iter() {
                    if let &DataItem::Str(_, ref ss) = i {
                        //println!("This name is [{}]", ss.trim());
                        let s = ss.trim();
                        let conf = ConfigFile::new(conf.database.to_string(), s.to_string());
                        databases.insert(s.to_string(), conf);
                    }
                }
            }
            system_db.commit_config();
        }
        Self {
            conf: conf,
            system: system,
            databases: databases,
        }
    }
    pub fn session(&mut self, user: &str, password: &str) -> Result<Session, String> {
        //登陆一个会话进程。
        let result = self.check_user(user, password);
        match result {
            Result::Ok(..) => {
                Result::Ok(Session {
                    engine: self,
                    using: Option::None,
                    user: user.to_string()
                })
            },
            Result::Err(ref e) => {
                Result::Err(e.to_string())
            }
        }
    }

    fn check_user(&mut self, user: &str, password: &str) -> Result<(), String> {
        let mut session = self.system.session();
        if let DResult::Table(ref dt) = session.select(&SelectSyntax{
            distinct: false,
            froms: hmap!["user" => Switch::One("user".to_string())],
            goals: vec![
                ("username".to_string(), Expression::new_single("username")),
                ("password".to_string(), Expression::new_single("password"))
            ],
            wheres: Expression{li: vec![
                ExpressionType::Var(vec!["username".to_string()]),
                ExpressionType::Str(user.to_string()),
                ExpressionType::Signal("=".to_string())
            ], setence: format!("username=\"{}\"", user)},
            orders: vec![]
        }) {
            if dt.content.len() == 1 {
                let record = dt.get_column("password");
                if let DataItem::Str(_, ref s) = record[0] {
                    if s == password {
                        Result::Ok(())
                    }else{
                        Result::Err(format!("Password wrong."))
                    }
                }else{
                    Result::Err(format!("External Error: wrong data type."))
                }
            }else{
                Result::Err(format!("User {} is not exists.", user))
            }
        }else{
            Result::Err(format!("External Error: cannot read user database."))
        }
    }

    pub fn create_database(&mut self, syntax:&CreateDatabaseSyntax) -> DResult {
        let name = syntax.name.as_str();
        if self.databases.contains_key(name) {
            DResult::String(format!("Database {} exists.", name))
        }else if name == "" {
            DResult::String(format!("Database name cannot be empty."))
        }else{
            self.databases.insert(name.to_string(), ConfigFile::new(self.conf.database.to_string(), name.to_string()));
            self.databases[name].save();
            let mut session = self.system.session();
            session.insert_into(&InsertSyntax{
                table_name: "database".to_string(),
                has_head: true,
                values: vec![hmap!["name"=>DfaWord::Str(name.to_string())]]
            });
            session.commit();
            DResult::String(format!("Database {} has created.", name))
        }
    }
    pub fn drop_database(&mut self, syntax:&DropDatabaseSyntax) -> DResult {
        let name = syntax.name.as_str();
        //要删除一个数据库，只需要从engine中清除数据库对象，从system表中删除数据库记录，并删除对应的实体文件。
        if ! self.databases.contains_key(name) {
            return DResult::String(format!("Database {} is not exists.", name));
        }
        let conf = self.databases.remove(name).unwrap();
        conf.delete_file();
        let mut session = self.system.session();
        session.delete(&DeleteSyntax{
            table_name: "database".to_string(),
            wheres: Expression{li: vec![
                ExpressionType::Var(vec!["name".to_string()]),
                ExpressionType::Str(name.to_string()),
                ExpressionType::Signal("=".to_string())
            ], setence: format!("name=\"{}\"", name)}
        });
        session.commit();
        DResult::String(format!("Database {} droped.", name))
    }

    pub fn create_user(&mut self, syntax: &CreateUserSyntax) -> DResult {
        let username = syntax.username.trim();
        if username == "" {
            return DResult::String(format!("Illegal user name."));
        }
        let mut session = self.system.session();
        let mut ok = false;
        if let DResult::Table(ref dt) = session.select(&SelectSyntax{
            distinct: false,
            froms: hmap!["user"=>Switch::One("user".to_string())],
            goals: vec![("username".to_string(), Expression::new_single("username"))],
            wheres: Expression{li:vec![
                ExpressionType::Var(vec!["username".to_string()]),
                ExpressionType::Str(username.to_string()),
                ExpressionType::Signal("=".to_string())
            ], setence: format!("username=\"{}\"", username)},
            orders: vec![]
        }) {
            if dt.content.len() > 0 {
                return DResult::String(format!("User {} is already exists.", username));
            }else{
                ok = true;
            }
        }
        if ok {
            session.insert_into(&InsertSyntax{
                table_name: "user".to_string(),
                has_head: true,
                values:vec![hmap![
                    "username" => DfaWord::Str(username.to_string()),
                    "password" => DfaWord::Str(syntax.password.trim().to_string()),
                    "is_root" => DfaWord::Bool(syntax.staff)
                ]]
            });
            session.commit();
            DResult::String(format!("User {} has created.", username))
        }else{
            DResult::String(format!("External error: cannot read user list."))
        }
    }
    pub fn alter_user(&mut self, syntax: &AlterUserSyntax) -> DResult {
        let username = syntax.username.trim();
        let mut session = self.system.session();
        let mut ok = false;
        if let DResult::Table(ref dt) = session.select(&SelectSyntax{
            distinct: false,
            froms: hmap!["user"=>Switch::One("user".to_string())],
            goals: vec![("username".to_string(), Expression::new_single("username"))],
            wheres: Expression{li:vec![
                ExpressionType::Var(vec!["username".to_string()]),
                ExpressionType::Str(username.to_string()),
                ExpressionType::Signal("=".to_string())
            ], setence: format!("username=\"{}\"", username)},
            orders: vec![]
        }) {
            if dt.content.len() <= 0 {
                return DResult::String(format!("User {} is not exists.", username));
            }else{
                ok = true;
            }
        }
        if ok {
            session.update(&UpdateSyntax{
                table_name: "user".to_string(),
                sets: hmap!["password" => DfaWord::Str(syntax.password.trim().to_string())],
                wheres: Expression{li:vec![
                    ExpressionType::Var(vec!["username".to_string()]),
                    ExpressionType::Str(username.to_string()),
                    ExpressionType::Signal("=".to_string())
                ], setence: format!("username=\"{}\"", username)}
            });
            session.commit();
            DResult::String(format!("User {} has been altered.", username))
        }else{
            DResult::String(format!("External error: cannot read user list."))
        }
    }
    pub fn drop_user(&mut self, syntax: &DropUserSyntax) -> DResult {
        let username = syntax.username.trim();
        if username == "" {return DResult::String(format!("User name cannot be empty."));}
        if username == "root" {return DResult::String(format!("Cannot delete root user."));}
        let mut session = self.system.session();
        let mut ok = false;
        if let DResult::Table(ref dt) = session.select(&SelectSyntax{
            distinct: false,
            froms: hmap!["user"=>Switch::One("user".to_string())],
            goals: vec![("username".to_string(), Expression::new_single("username"))],
            wheres: Expression{li:vec![
                ExpressionType::Var(vec!["username".to_string()]),
                ExpressionType::Str(username.to_string()),
                ExpressionType::Signal("=".to_string())
            ], setence: format!("username=\"{}\"", username)},
            orders: vec![]
        }) {
            if dt.content.len() <= 0 {
                return DResult::String(format!("User {} is not exists.", username));
            }else{
                ok = true;
            }
        }
        if ok {
            session.delete(&DeleteSyntax{
                table_name: "user".to_string(),
                wheres: Expression{li:vec![
                    ExpressionType::Var(vec!["username".to_string()]),
                    ExpressionType::Str(username.to_string()),
                    ExpressionType::Signal("=".to_string())
                ], setence: format!("username=\"{}\"", username)}
            });
            session.commit();
            DResult::String(format!("User {} has been droped.", username))
        }else{
            DResult::String(format!("External error: cannot read user list."))
        }
    }

    pub fn grant(&mut self, syntax: &GrantSyntax, db: &str) -> DResult {
        //grant语句赋予或收回权限给目标。
        let mut session = self.system.session();
        if syntax.is_grant {
            let mut values = Vec::new();
            for user in syntax.users.iter() {
                for &(ref t, ref obj) in syntax.objects.iter() {
                    if t == "table" && db == "" {continue;}
                    if syntax.all {
                        values.push(hmap![
                            "username" => DfaWord::Str(user.to_string()),
                            "database" => DfaWord::Str(if t == "database" {obj.to_string()}else{db.to_string()}),
                            "table" => DfaWord::Str(if t == "database" {"".to_string()}else{obj.to_string()}),
                            "type" => DfaWord::Str("all".to_string())
                        ]);
                    }else {
                        for grant in syntax.grants.iter() {
                            values.push(hmap![
                                "username" => DfaWord::Str(user.to_string()),
                                "database" => DfaWord::Str(if t == "database" {obj.to_string()}else{db.to_string()}),
                                "table" => DfaWord::Str(if t == "database" {"".to_string()}else{obj.to_string()}),
                                "type" => DfaWord::Str(grant.to_string())
                            ]);
                        }
                    }
                }
            }
            session.insert_into(&InsertSyntax{
                table_name: "privilege".to_string(),
                has_head: true,
                values: values
            });
            session.commit();
            DResult::String(format!("Grant complete."))
        }else{
            let mut values = Vec::new();
            for user in syntax.users.iter() {
                for &(ref t, ref obj) in syntax.objects.iter() {
                    if t == "table" && db == "" {continue;}
                    if syntax.all {
                        values.push(hmap![
                            "username" => user.to_string(),
                            "database" => if t == "database" {obj.to_string()}else{db.to_string()},
                            "table" => if t == "database" {"".to_string()}else{obj.to_string()},
                            "type" => "all".to_string()
                        ]);
                    }else {
                        for grant in syntax.grants.iter() {
                            values.push(hmap![
                                "username" => user.to_string(),
                                "database" => if t == "database" {obj.to_string()}else{db.to_string()},
                                "table" => if t == "database" {"".to_string()}else{obj.to_string()},
                                "type" => grant.to_string()
                            ]);
                        }
                    }
                }
            }
            for i in values.iter() {
                session.delete(&DeleteSyntax{
                    table_name: "privilege".to_string(),
                    wheres: Expression{li:vec![
                        ExpressionType::Var(vec!["username".to_string()]),
                        ExpressionType::Str(i["username"].to_string()),
                        ExpressionType::Signal("=".to_string()),
                        ExpressionType::Var(vec!["database".to_string()]),
                        ExpressionType::Str(i["database"].to_string()),
                        ExpressionType::Signal("=".to_string()),
                        ExpressionType::Signal("&&".to_string()),
                        ExpressionType::Var(vec!["table".to_string()]),
                        ExpressionType::Str(i["table"].to_string()),
                        ExpressionType::Signal("=".to_string()),
                        ExpressionType::Signal("&&".to_string()),
                        ExpressionType::Var(vec!["type".to_string()]),
                        ExpressionType::Str(i["type"].to_string()),
                        ExpressionType::Signal("=".to_string()),
                        ExpressionType::Signal("&&".to_string())
                    ], setence: format!("username=\"{}\"&&database=\"{}\"&&table=\"{}\"&&type=\"{}\"", i["username"], i["database"], i["table"], i["type"])}
                });
            }
            session.commit();
            DResult::String(format!("Revoke complete."))
        }
        
    }
}

pub struct Session<'t>{
    engine:&'t mut Engine,
    using: Option<String>,
    user: String
}
impl<'t> Session<'t> {
    pub fn use_database(&mut self, syntax:&UseSyntax) -> DResult {
        let name = syntax.name.as_str();
        // todo 检查权限
        if self.engine.databases.contains_key(name) {
            self.using = Option::Some(name.to_string());
            DResult::String(format!("use {}.", name))
        }else {
            DResult::String(format!("Database {} is not exists.", name))
        }
    }

    fn get_using(&mut self) -> Result<Database, DResult> {
        if let Option::Some(ref db) = self.using {
            if ! self.engine.databases.contains_key(db) {
                Result::Err(DResult::String(format!("No using database.")))
            }else{
                Result::Ok(self.engine.databases.get_mut(db).unwrap().session())
            }
        } else {Result::Err(DResult::String(format!("No using database.")))}
    }

    pub fn execute(&mut self, syntax:&ColSyntax) -> DResult {
        if let Result::Err(ref e) = self.check_grant(syntax) {
            return DResult::String(e.to_string());
        }
        match syntax {
            &ColSyntax::Select(ref s) => match self.get_using() {
                Result::Ok(mut db) => db.select(s),
                Result::Err(dr) => dr
            },
            &ColSyntax::Insert(ref s) => match self.get_using() {
                Result::Ok(mut db) => {
                    let ret = db.insert_into(s);
                    db.commit();
                    ret
                },
                Result::Err(dr) => dr
            },
            &ColSyntax::Update(ref s) => match self.get_using() {
                Result::Ok(mut db) => {
                    let ret = db.update(s);
                    db.commit();
                    ret
                },
                Result::Err(dr) => dr
            },
            &ColSyntax::Delete(ref s) => match self.get_using() {
                Result::Ok(mut db) => {
                    let ret = db.delete(s);
                    db.commit();
                    ret
                },
                Result::Err(dr) => dr
            },
            &ColSyntax::CreateTable(ref s) => match self.get_using() {
                Result::Ok(mut db) => {
                    let ret = db.create_table(s);
                    db.commit();
                    ret
                },
                Result::Err(dr) => dr
            },
            &ColSyntax::AlterTable(ref s) => match self.get_using() {
                Result::Ok(mut db) => {
                    let ret = db.alter_table(s);
                    db.commit();
                    ret
                },
                Result::Err(dr) => dr
            },
            &ColSyntax::DropTable(ref s) => match self.get_using() {
                Result::Ok(mut db) => {
                    let ret = db.drop_table(s);
                    db.commit();
                    ret
                },
                Result::Err(dr) => dr
            },
            &ColSyntax::CreateView(ref s) => match self.get_using() {
                Result::Ok(mut db) => {
                    let ret = db.create_view(s);
                    db.commit();
                    ret
                },
                Result::Err(dr) => dr
            },
            &ColSyntax::DropView(ref s) => match self.get_using() {
                Result::Ok(mut db) => {
                    let ret = db.drop_view(s);
                    db.commit();
                    ret
                },
                Result::Err(dr) => dr
            },
            &ColSyntax::Use(ref s) => self.use_database(s),
            &ColSyntax::Help(ref s) => match self.get_using() {
                Result::Ok(mut db) => db.help(s),
                Result::Err(dr) => dr
            },
            &ColSyntax::CreateDatabase(ref s) => self.engine.create_database(s),
            &ColSyntax::DropDatabase(ref s) => self.engine.drop_database(s),
            &ColSyntax::CreateUser(ref s) => self.engine.create_user(s),
            &ColSyntax::AlterUser(ref s) => self.engine.alter_user(s),
            &ColSyntax::DropUser(ref s) => self.engine.drop_user(s),
            &ColSyntax::Grant(ref s) => self.engine.grant(s, if let Option::Some(ref s) = self.using{s}else{""}),
            &ColSyntax::None => {
                DResult::String(format!("无效的指令。"))
            }
        }
    }

    pub fn check_grant(&mut self, syntax:&ColSyntax) -> Result<(), String> {
        //检查当前用户是否具有某项操作的权限。
        //对于user/grant系列语句，必须是staff。
        //对于database序列语句，必须是staff。
        //对于table操作/update/insert/delete/help语句，使用权限库直接检查对应表数据库.
        //对于select语句，递归检查对应表/数据库
        //staff对所有的语句具有访问权。
        let staff = match self.is_staff() {
            Result::Ok(ok) => ok,
            Result::Err(ref e) => {return Result::Err(e.to_string());}
        };
        if staff {
            return Result::Ok(());
        }
        //从这里开始的判定对视已经没有staff的了。
        match syntax {
            &ColSyntax::Grant(..) |
            &ColSyntax::CreateUser(..) | 
            &ColSyntax::AlterUser(..) |
            &ColSyntax::DropUser(..) |
            &ColSyntax::CreateDatabase(..) |
            &ColSyntax::DropDatabase(..) => {
                Result::Err(format!("You do not have grant on these setences."))
            },
            &ColSyntax::Use(ref s) => {
                match self.has_any_grant(s.name.as_str()) {
                    Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                    Result::Err(ref e) => Result::Err(e.to_string())
                }
            },
            //下面的语句都是use依赖的语句
            &ColSyntax::CreateTable(_) => {
                let db = match self.using {
                    Option::Some(ref db) => db.to_string(), 
                    Option::None => {return Result::Ok(());}  //在没有use时是默认可以的，反正也执行不了
                };
                match self.has_grant_on_database(db.as_str(), "createtable") {
                    Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                    Result::Err(ref e) => Result::Err(e.to_string())
                }
            },
            &ColSyntax::AlterTable(_) => {
                let db = match self.using {
                    Option::Some(ref db) => db.to_string(), 
                    Option::None => {return Result::Ok(());}  //在没有use时是默认可以的，反正也执行不了
                };
                match self.has_grant_on_database(db.as_str(), "altertable") {
                    Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                    Result::Err(ref e) => Result::Err(e.to_string())
                }
            },
            &ColSyntax::DropTable(_) => {
                let db = match self.using {
                    Option::Some(ref db) => db.to_string(), 
                    Option::None => {return Result::Ok(());}  //在没有use时是默认可以的，反正也执行不了
                };
                match self.has_grant_on_database(db.as_str(), "droptable") {
                    Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                    Result::Err(ref e) => Result::Err(e.to_string())
                }
            },
            &ColSyntax::CreateView(_) => {
                let db = match self.using {
                    Option::Some(ref db) => db.to_string(), 
                    Option::None => {return Result::Ok(());}  //在没有use时是默认可以的，反正也执行不了
                };
                match self.has_grant_on_database(db.as_str(), "createview") {
                    Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                    Result::Err(ref e) => Result::Err(e.to_string())
                }
            },
            &ColSyntax::DropView(_) => {
                let db = match self.using {
                    Option::Some(ref db) => db.to_string(), 
                    Option::None => {return Result::Ok(());}  //在没有use时是默认可以的，反正也执行不了
                };
                match self.has_grant_on_database(db.as_str(), "dropview") {
                    Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                    Result::Err(ref e) => Result::Err(e.to_string())
                }
            }
            &ColSyntax::Help(ref s) => {
                //help的权限进行分割放送。
                let db = match self.using {
                    Option::Some(ref db) => db.to_string(), 
                    Option::None => {return Result::Ok(());}  //在没有use时是默认可以的，反正也执行不了
                };
                match s.params[0].as_str() {
                    "database" => match self.has_grant_on_database(db.as_str(), "help") {
                        Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                        Result::Err(ref e) => Result::Err(e.to_string())
                    },
                    "table" => match self.has_grant_on_table(db.as_str(), s.params[1].as_str(), "help") {
                        Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                        Result::Err(ref e) => Result::Err(e.to_string())
                    },
                    "view" => match self.has_grant_on_table(db.as_str(), s.params[1].as_str(), "help") {
                        Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                        Result::Err(ref e) => Result::Err(e.to_string())
                    },
                    // todo index和view在这里需要添加。
                    _ => Result::Err(format!("Unknown help syntax."))
                }
            },
            &ColSyntax::Update(ref s) => {
                let db = match self.using {
                    Option::Some(ref db) => db.to_string(), 
                    Option::None => {return Result::Ok(());}  //在没有use时是默认可以的，反正也执行不了
                };
                match self.has_grant_on_table(db.as_str(), s.table_name.as_str(), "update") {
                    Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                    Result::Err(ref e) => Result::Err(e.to_string())
                }
            },
            &ColSyntax::Insert(ref s) => {
                let db = match self.using {
                    Option::Some(ref db) => db.to_string(), 
                    Option::None => {return Result::Ok(());}  //在没有use时是默认可以的，反正也执行不了
                };
                match self.has_grant_on_table(db.as_str(), s.table_name.as_str(), "insert") {
                    Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                    Result::Err(ref e) => Result::Err(e.to_string())
                }
            },
            &ColSyntax::Delete(ref s) => {
                let db = match self.using {
                    Option::Some(ref db) => db.to_string(), 
                    Option::None => {return Result::Ok(());}  //在没有use时是默认可以的，反正也执行不了
                };
                match self.has_grant_on_table(db.as_str(), s.table_name.as_str(), "delete") {
                    Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                    Result::Err(ref e) => Result::Err(e.to_string())
                }
            },
            &ColSyntax::Select(ref s) => {
                let db = match self.using {
                    Option::Some(ref db) => db.to_string(), 
                    Option::None => {return Result::Ok(());}  //在没有use时是默认可以的，反正也执行不了
                };
                //select的语法要麻烦一些，因为存在嵌套的查询，表名不直观。
                let mut tlist = Vec::new();
                self.get_tables_names(&mut tlist, s);
                match self.has_grant_on_tables(db.as_str(), &tlist[..], "select") {
                    Result::Ok(ok) => if ok {Result::Ok(())}else{Result::Err(format!("You do not have grant on these setences."))},
                    Result::Err(ref e) => Result::Err(e.to_string())
                }
            },
            &ColSyntax::None => {
                Result::Ok(())
            }
        }        
    }
    fn get_tables_names(&self, tlist:&mut Vec<String>, syntax:&SelectSyntax) {
        for (_, i) in syntax.froms.iter() {
            match i {
                &Switch::One(ref s) => {tlist.push(s.to_string());},
                &Switch::Two(ref s) => {self.get_tables_names(tlist, s);}
            }
        }
    }
    fn has_any_grant(&mut self, database:&str) -> Result<bool, String> {
        if !self.engine.databases.contains_key(database) {
            return Result::Ok(true);  //不存在的数据库是被允许的。
        }
        let mut session = self.engine.system.session();
        if let DResult::Table(ref dt) = session.select(&SelectSyntax{
            distinct: false,
            froms: hmap!["privilege"=>Switch::One("privilege".to_string())],
            goals: vec![("*".to_string(), Expression::new_allin())],
            wheres: Expression{li:vec![
                ExpressionType::Var(vec!["username".to_string()]),
                ExpressionType::Str(self.user.to_string()),
                ExpressionType::Signal("=".to_string()),
                ExpressionType::Var(vec!["database".to_string()]),
                ExpressionType::Str(database.to_string()),
                ExpressionType::Signal("=".to_string()),
                ExpressionType::Signal("&&".to_string())
            ], setence: format!("username=\"{}\"&&database=\"{}\"", self.user, database)},
            orders: vec![]
        }) { //筛选出与user相关的所有与当前数据库相关的权限记录。
            Result::Ok(dt.content.len() > 0)
            //只要存在记录，就表示有任意权限存在。
        }else{
            Result::Err(format!("External error: cannot read privileges list."))
        }
    }
    fn has_grant_on_database(&mut self, database:&str, grant:&str) -> Result<bool, String> {
        if !self.engine.databases.contains_key(database) {
            return Result::Ok(true);  //不存在的数据库是被允许的。
        }
        //判断用户对该数据库是否具有grant的权限，或者具有all权限。
        let mut session = self.engine.system.session();
        if let DResult::Table(ref dt) = session.select(&SelectSyntax{
            distinct: false,
            froms: hmap!["privilege"=>Switch::One("privilege".to_string())],
            goals: vec![("*".to_string(), Expression::new_allin())],
            wheres: Expression{li:vec![
                ExpressionType::Var(vec!["username".to_string()]),
                ExpressionType::Str(self.user.to_string()),
                ExpressionType::Signal("=".to_string()),
                ExpressionType::Var(vec!["database".to_string()]),
                ExpressionType::Str(database.to_string()),
                ExpressionType::Signal("=".to_string()),
                ExpressionType::Signal("&&".to_string()),
                ExpressionType::Var(vec!["table".to_string()]),
                ExpressionType::Str("".to_string()),
                ExpressionType::Signal("=".to_string()),
                ExpressionType::Signal("&&".to_string())
            ], setence: format!("username=\"{}\"&&database=\"{}\"&&table=\"\"", self.user, database)},
            orders: vec![]
        }) { //筛选出与user相关的所有与当前数据库相关的权限记录，去掉table不为空的记录，只保留针对database本身的记录。
            if dt.content.len() > 0 {
                let record = dt.get_column("type");
                for i in record.iter() {
                    if let &DataItem::Str(_, ref s) = i {
                        if s == grant || s == "all" {
                            return Result::Ok(true)
                        }
                    }
                }
            }
            Result::Ok(false)
        }else{
            Result::Err(format!("External error: cannot read privileges list."))
        }
    }
    fn has_grant_on_table(&mut self, database:&str, table:&str, grant:&str) -> Result<bool, String> {
        if !self.engine.databases.contains_key(database) {
            return Result::Ok(true);  //不存在的数据库是被允许的。
        }
        //判断用户对该表否具有grant的权限，或者具有all权限。
        let mut session = self.engine.system.session();
        if let DResult::Table(ref dt) = session.select(&SelectSyntax{
            distinct: false,
            froms: hmap!["privilege"=>Switch::One("privilege".to_string())],
            goals: vec![("*".to_string(), Expression::new_allin())],
            wheres: Expression{li:vec![
                ExpressionType::Var(vec!["table".to_string()]),
                ExpressionType::Str(table.to_string()),
                ExpressionType::Signal("=".to_string()),
                ExpressionType::Var(vec!["table".to_string()]),
                ExpressionType::Str("".to_string()),
                ExpressionType::Signal("=".to_string()),
                ExpressionType::Signal("||".to_string()),
                ExpressionType::Var(vec!["username".to_string()]),
                ExpressionType::Str(self.user.to_string()),
                ExpressionType::Signal("=".to_string()),
                ExpressionType::Signal("&&".to_string()),
                ExpressionType::Var(vec!["database".to_string()]),
                ExpressionType::Str(database.to_string()),
                ExpressionType::Signal("=".to_string()),
                ExpressionType::Signal("&&".to_string())
            ],setence: format!(
                "table=\"{}\"||table=\"\"&&username=\"{}\"&&database=\"{}\"",
                table, self.user, database
            )},
            orders: vec![]
        }) { //筛选出与user相关的所有与当前表或当前数据库相关的权限记录
            if dt.content.len() > 0 {
                let record = dt.get_column("type");
                for i in record.iter() {
                    if let &DataItem::Str(_, ref s) = i {
                        if s == grant || s == "all" {
                            return Result::Ok(true)
                        }
                    }
                }
            }
            Result::Ok(false)
        }else{
            Result::Err(format!("External error: cannot read privileges list."))
        }
    }
    fn has_grant_on_tables(&mut self, database:&str, tables:&[String], grant:&str) -> Result<bool, String> {
        if !self.engine.databases.contains_key(database) {
            return Result::Ok(true);  //不存在的数据库是被允许的。
        }
        //判断用户对该表否具有grant的权限，或者具有all权限。
        let mut session = self.engine.system.session();
        if let DResult::Table(ref dt) = session.select(&SelectSyntax{
            distinct: false,
            froms: hmap!["privilege"=>Switch::One("privilege".to_string())],
            goals: vec![
                ("table".to_string(), Expression::new_single("table")),
                ("type".to_string(), Expression::new_single("type"))
            ],
            wheres: Expression{li:vec![
                ExpressionType::Var(vec!["username".to_string()]),
                ExpressionType::Str(self.user.to_string()),
                ExpressionType::Signal("=".to_string()),
                ExpressionType::Var(vec!["database".to_string()]),
                ExpressionType::Str(database.to_string()),
                ExpressionType::Signal("=".to_string()),
                ExpressionType::Signal("&&".to_string())
            ], setence: format!("username=\"{}\"&&database=\"{}\"", self.user, database)},
            orders: vec![]
        }) { //筛选出与user相关的所有与当前表或当前数据库相关的权限记录
            for d in dt.content.iter() {
                for i in tables.iter() {
                    if let DataItem::Str(_, ref s) = d.li[0] {
                        if s == i || s == "" {
                            //确认表名
                            if let DataItem::Str(_, ref s) = d.li[1] {
                                if s == grant || s == "all" {
                                    return Result::Ok(true);
                                }
                            }
                        }
                    }
                }
            }
            Result::Ok(false)
        }else{
            Result::Err(format!("External error: cannot read privileges list."))
        }
    } 
    fn is_staff(&mut self) -> Result<bool, String> {
        let mut session = self.engine.system.session();
        if let DResult::Table(ref dt) = session.select(&SelectSyntax{
            distinct: false,
            froms: hmap!["user"=>Switch::One("user".to_string())],
            goals: vec![
                ("username".to_string(), Expression::new_single("username")),
                ("staff".to_string(), Expression::new_single("is_root"))
            ],
            wheres: Expression{li:vec![
                ExpressionType::Var(vec!["username".to_string()]),
                ExpressionType::Str(self.user.to_string()),
                ExpressionType::Signal("=".to_string())
            ], setence: format!("username=\"{}\"", self.user)},
            orders: vec![]
        }) {
            if dt.content.len() != 1 {
                return Result::Err(format!("External error: wrong result of user list."));
            }
            let records = dt.get_column("staff");
            let staff: bool = if let DataItem::Bool(b) = records[0] {b}else{
                return Result::Err(format!("External error: wrong result type."));
            };
            Result::Ok(staff)
        }else{
            Result::Err(format!("External error: cannot read user list."))
        }
    }

    pub fn get_username(&self) -> String {
        self.user.to_string()
    }
    pub fn get_using_database(&self) -> Option<String> {
        if let Option::Some(ref s) = self.using {
            Option::Some(s.to_string())
        }else{
            Option::None
        }
    }

}