extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::collections::{HashMap, BTreeMap};
use regex::{Regex, Captures};

#[derive(Deserialize, Debug)]
struct RolePermissions {
    copy: Option<String>,
    perms: Option<Vec<String>>,
    all_but: Option<Vec<String>>,
}
#[derive(Deserialize, Debug)]
struct Permissions {
    roles: BTreeMap<String, HashMap<String, String>>,
    permissions: Vec<String>,
    role_permissions: HashMap<String, RolePermissions>,
}

/// We're going to statically generate some rust code to reflect our heroic
/// permissions.
fn main() {
    // we're going to turn a lot of lisp-type-ids into RustStyleCamelCase
    let re_camel_case = Regex::new("(^|-)([a-z])").unwrap();

    // this helpful macro lets us inline our json! thanks, rust.
    let json: &'static str = include_str!("./permissions.json");
    // parse the json into a permissions object
    let permissions: Permissions = serde_json::from_str(&String::from(json)).unwrap();

    // this will hold our final output!
    let mut output: String = String::new();

    output.push_str("\n");

    // create a Role enum with all our roles
    output.push_str("#[derive(Serialize, Deserialize, Debug)]\n");
    output.push_str("pub enum Role {\n");
    for kv in &permissions.roles {
        let camel = re_camel_case.replace_all(kv.0, |caps: &Captures| {
            format!("{}", &caps[2]).to_uppercase()
        });
        // be sure we can (de)serialize with the names from json...this is
        // important for interoperability with the server
        output.push_str(format!("    #[serde(rename = \"{}\")]\n", kv.0).as_str());
        output.push_str(format!("    {},\n", camel.as_str()).as_str());
    }
    output.push_str("}\n");

    output.push_str("\n");

    // implement our stupid role.
    output.push_str("impl Role {\n");

    // a function that returns a list of each role along with its stupid
    // description. useful for sending a list of roles to, oh, i don't know,
    // the UI???
    output.push_str("    pub fn all_roles() -> Vec<(Role, &'static str)> {\n");
    output.push_str(format!("        let mut roles = Vec::with_capacity({});\n", permissions.roles.keys().len()).as_str());
    for kv in &permissions.roles {
        let camel = re_camel_case.replace_all(kv.0, |caps: &Captures| {
            format!("{}", &caps[2]).to_uppercase()
        });
        output.push_str(format!("        let role = Role::{};\n", camel).as_str());
        output.push_str("        let desc = role.desc();\n");
        output.push_str("        roles.push((role, desc));\n");
    }
    output.push_str("        roles\n");
    output.push_str("    }\n");
    output.push_str("\n");

    // create a desc() function that returns this role's description text
    output.push_str("    pub fn desc(&self) -> &'static str {\n");
    output.push_str("        match self {\n");
    for kv in &permissions.roles {
        let camel = re_camel_case.replace_all(kv.0, |caps: &Captures| {
            format!("{}", &caps[2]).to_uppercase()
        });
        let desc = kv.1.get(&String::from("desc")).unwrap();
        output.push_str(format!("            &Role::{} => \"{}\",\n", camel, desc).as_str());
    }
    output.push_str("        }\n");
    output.push_str("    }\n");
    output.push_str("\n");

    // here we build our role <--> permission map. it's a hash table where each
    // key is a role (lisp-type, not CamelCase) which points to a vector of
    // permission names (also-lisp-typed);
    let mut role_permissions: HashMap<String, Vec<String>> = HashMap::new();
    for kv in &permissions.role_permissions {
        // create an empty vec for each role
        let mut rp = role_permissions.entry(kv.0.clone()).or_insert(Vec::new());
        // now, process our `all_but` key
        match kv.1.all_but.as_ref() {
            Some(all_but) => {
                for perm in &permissions.permissions {
                    if all_but.contains(perm) { continue; }
                    rp.push(perm.clone());
                }
            }
            None => {}
        }
    }
    // now loop over the role permissions again, this time processing our `copy`
    // and our `perms` directives.
    for kv in &permissions.role_permissions {
        match kv.1.copy.as_ref() {
            Some(copy) => {
                let copied = role_permissions.get(copy).unwrap().clone();
                let mut rp = role_permissions.entry(kv.0.clone()).or_insert(Vec::new());
                for role in copied {
                    rp.push(role);
                }
            }
            None => {}
        }
        match kv.1.perms.as_ref() {
            Some(perms) => {
                let mut rp = role_permissions.entry(kv.0.clone()).or_insert(Vec::new());
                for perm in perms {
                    rp.push(perm.clone());
                }
            }
            None => {}
        }
    }

    // great! we have our role <--> permission map! now build a function that
    // takes a permission and tells us whether or not the current role can
    // perform that action.
    output.push_str("    pub fn can(&self, permission: &Permission) -> bool {\n");
    output.push_str("        match *self {\n");
    for role in &permissions.roles {
        let camel = re_camel_case.replace_all(role.0, |caps: &Captures| {
            format!("{}", &caps[2]).to_uppercase()
        });
        output.push_str(format!("            Role::{} => {{\n", camel).as_str());
        output.push_str(format!("                match permission {{\n").as_str());
        let rp = role_permissions.get(role.0).unwrap();
        for perm in &permissions.permissions {
            let camel = re_camel_case.replace_all(perm, |caps: &Captures| {
                format!("{}", &caps[2]).to_uppercase()
            });
            let truefalse = if rp.contains(perm) { "true" } else { "false" };
            output.push_str(format!("                    &Permission::{} => {},\n", camel, truefalse).as_str());
        }
        output.push_str(format!("                }}\n").as_str());
        output.push_str(format!("            }}\n").as_str());
    }
    output.push_str("        }\n");
    output.push_str("    }\n");
    output.push_str("\n");

    // build a function that, given a role, returns a vec of actions
    // (permissions) that role can perform
    output.push_str("    pub fn allowed_permissions(&self) -> Vec<Permission> {\n");
    output.push_str("        match *self {\n");
    for role in &permissions.roles {
        let camel = re_camel_case.replace_all(role.0, |caps: &Captures| {
            format!("{}", &caps[2]).to_uppercase()
        });
        output.push_str(format!("            Role::{} => {{\n", camel).as_str());
        output.push_str(format!("                vec![\n").as_str());
        let rp = role_permissions.get(role.0).unwrap();
        for perm in &permissions.permissions {
            if !rp.contains(perm) { continue; }
            let camel = re_camel_case.replace_all(perm, |caps: &Captures| {
                format!("{}", &caps[2]).to_uppercase()
            });
            output.push_str(format!("                    Permission::{},\n", camel).as_str());
        }
        output.push_str(format!("                ]\n").as_str());
        output.push_str(format!("            }}\n").as_str());
    }
    output.push_str("        }\n");
    output.push_str("    }\n");

    output.push_str("}\n");

    output.push_str("\n");

    // now create an enum with all our permissions
    output.push_str("#[derive(Serialize, Deserialize, Debug)]\n");
    output.push_str("pub enum Permission {\n");
    for perm in &permissions.permissions {
        let rep = re_camel_case.replace_all(perm.as_str(), |caps: &Captures| {
            format!("{}", &caps[2]).to_uppercase()
        });
        output.push_str(format!("    #[serde(rename = \"{}\")]\n", perm).as_str());
        output.push_str(format!("    {},\n", rep).as_str());
    }
    output.push_str("}\n");

    output.push_str("\n");

    // write it all out to our src/gen.rs file, included by lib
    let out_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut dest_path = PathBuf::from(&out_dir);
    dest_path.push(String::from("src"));
    dest_path.push(String::from("gen.rs"));
    let mut f = File::create(&dest_path).unwrap();
    f.write_all(output.as_bytes()).unwrap();
}

