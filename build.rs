extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::collections::HashMap;
use regex::{Regex, Captures};

#[derive(Deserialize, Debug)]
struct RolePermissions {
    copy: Option<String>,
    perms: Option<Vec<String>>,
    all_but: Option<Vec<String>>,
}
#[derive(Deserialize, Debug)]
struct Permissions {
    roles: HashMap<String, HashMap<String, String>>,
    permissions: Vec<String>,
    role_permissions: HashMap<String, RolePermissions>,
}

// {
//   "roles": {
//     "owner": "owner",
//     "admin": "admin",
//     "moderator": "moderator",
//     "member": "member",
//     "guest": "guest"
//   },
//   "permissions": {
//     "edit_space": "edit-space",
//     "delete_space": "delete-space",
//     "set_space_owner": "set-space-owner",
//     "edit_space_member": "edit-space-member",
//     "delete_space_member": "delete-space-member",
//     "add_space_invite": "add-space-invite",
//     "edit_space_invite": "edit-space-invite",
//     "delete_space_invite": "delete-space-invite",
//     "add_board": "add-board",
//     "edit_board": "edit-board",
//     "delete_board": "delete-board",
//     "add_note": "add-note",
//     "edit_note": "edit-note",
//     "delete_note": "delete-note"
//   },
//   "role_permissions": {
//     "owner": [
//       "edit-space",
//       "edit-space-member",
//       "delete-space-member",
//       "add-space-invite",
//       "edit-space-invite",
//       "delete-space-invite",
//       "add-board",
//       "edit-board",
//       "delete-board",
//       "add-note",
//       "edit-note",
//       "delete-note",
//       "set-space-owner",
//       "delete-space"
//     ],
//     "admin": [
//       "edit-space",
//       "edit-space-member",
//       "delete-space-member",
//       "add-space-invite",
//       "edit-space-invite",
//       "delete-space-invite",
//       "add-board",
//       "edit-board",
//       "delete-board",
//       "add-note",
//       "edit-note",
//       "delete-note"
//     ],
//     "moderator": [
//       "add-board",
//       "edit-board",
//       "delete-board",
//       "add-note",
//       "edit-note",
//       "delete-note"
//     ],
//     "member": [
//       "add-note",
//       "edit-note",
//       "delete-note"
//     ],
//     "guest": []
//   },
//   "desc": {
//     "owner": "Can do anything.",
//     "admin": "Can invite and moderate users, edit boards and notes.",
//     "moderator": "Can edit boards and notes.",
//     "member": "Can edit notes.",
//     "guest": "Read-only."
//   }
// }

/// We're going to statically generate some rust code to reflect our heroic
/// permissions.
fn main() {
    let re_camel_case = Regex::new("(^|-)([a-z])").unwrap();
    let json: &'static str = include_str!("./permissions.json");
    let permissions: Permissions = serde_json::from_str(&String::from(json)).unwrap();
    let mut output: String = String::new();

    output.push_str("\n");

    output.push_str("#[derive(Serialize, Deserialize, Debug)]\n");
    output.push_str("pub enum Role {\n");
    for kv in &permissions.roles {
        let camel = re_camel_case.replace_all(kv.0, |caps: &Captures| {
            format!("{}", &caps[2]).to_uppercase()
        });
        output.push_str(format!("    #[serde(rename = \"{}\")]\n", kv.0).as_str());
        output.push_str(format!("    {},\n", camel.as_str()).as_str());
    }
    output.push_str("}\n");

    output.push_str("\n");

    let mut role_permissions: HashMap<String, Vec<String>> = HashMap::new();
    for kv in &permissions.role_permissions {
        let mut rp = role_permissions.entry(kv.0.clone()).or_insert(Vec::new());
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

    output.push_str("impl Role {\n");
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
    output.push_str("}\n");

    output.push_str("\n");

    output.push_str("pub enum Permission {\n");
    for perm in &permissions.permissions {
        let rep = re_camel_case.replace_all(perm.as_str(), |caps: &Captures| {
            format!("{}", &caps[2]).to_uppercase()
        });
        output.push_str("    ");
        output.push_str(rep.as_str());
        output.push_str(",\n");
    }
    output.push_str("}\n");

    output.push_str("\n");

    let out_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut dest_path = PathBuf::from(&out_dir);
    dest_path.push(String::from("src"));
    dest_path.push(String::from("gen.rs"));
    let mut f = File::create(&dest_path).unwrap();
    f.write_all(output.as_bytes()).unwrap();
}

