extern crate serde;
#[macro_use]
extern crate serde_derive;

mod gen;
pub use gen::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_perms() {
        Role::all_roles();
    }
}

