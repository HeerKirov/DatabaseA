extern crate dba;
use std::io::{self, Write};
use std::env;
use dba::analyse::dfa::{FiniteAutomaton};
use dba::syntax::toptree::{PublicTree};
use dba::syntax::structures::{UseSyntax};
use dba::engine::engine::{Engine};

fn get_env() -> (String, String, String) {
    //从env中获取启动参数，并返回。
    //依次为：(-u)user, (-p)passwd, (-d)database.
    let mut argument = vec![];
    for i in env::args() {argument.push(i);}
    let mut user = "";
    let mut pw = "";
    let mut db = "";
    let mut i = 1;
    while i < argument.len() {
        let s = &argument[i].to_lowercase();
        if s == "-u" && user == "" {
            user = &argument[i+1];
        }else if s == "-p" && pw == "" {
            pw = &argument[i+1];
        }else if s == "-d" && db == "" {
            db = &argument[i+1];
        }
        i += 1;
    }
    (user.to_string(), pw.to_string(), db.to_string())
}
fn get_user_runtime(user:&mut String, pw:&mut String) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    
    handle.write(b"Login as:").unwrap();
    handle.flush().unwrap();
    {
        let stdin = io::stdin();
        let mut input = String::new();
        if let Result::Ok(..) = stdin.read_line(&mut input) {
            *user = input.trim().to_string();
        }
    }
    handle.write(b"Password:").unwrap();
    handle.flush().unwrap();
    {
        let stdin = io::stdin();
        let mut input = String::new();
        if let Result::Ok(..) = stdin.read_line(&mut input) {
            *pw = input.trim().to_string();
        }
    }
} 
fn main() {
    let mut engine = Engine::new();
    let (mut user, mut password, db_name) = get_env();
    if user == "" || password == "" {
        get_user_runtime(&mut user, &mut password);
    }
    let stdout = io::stdout();
    let stdin = io::stdin();
    let mut session = match engine.session(&user, &password) {
        Result::Ok(ok) => ok,
        Result::Err(e) => {
            stdout.lock().write(format!("{}", e).as_bytes()).unwrap();
            return;
        }
    };
    if db_name != "" {
        session.use_database(&UseSyntax::new(&db_name));
    }
    loop {
        {
            let mut handle = stdout.lock();
            handle.write(format!("\n{}[{}]>", 
                session.get_username(), 
                if let Option::Some(ref s) = session.get_using_database(){s}else{"None"}
            ).as_bytes()).unwrap();
            handle.flush().unwrap();
        }
        let mut input = String::new();
        if let Result::Ok(..) = stdin.read_line(&mut input) {
            if input.to_lowercase().trim() == "exit".to_string() {
                stdout.lock().write(format!("Bye.").as_bytes()).unwrap();
                break;
            }
            let mut fa = FiniteAutomaton::new(input);
            let dfawords = fa.construct();
            if let Option::Some(ref s) = fa.get_error_string() {
                stdout.lock().write(format!("ERROR: {}", s).as_bytes()).unwrap();
                continue;
            }
            let mut tree = PublicTree::new();
            let result = tree.construct(&dfawords[..]);
            if let Option::Some(ref s) = tree.get_error_string() {
                stdout.lock().write(format!("ERROR: {}", s).as_bytes()).unwrap();
                continue;
            }
            let response = session.execute(&result);
            response.print(&mut stdout.lock());
        }else{
            stdout.lock().write(format!("Bye.").as_bytes()).unwrap();
            break;
        }
    }
}