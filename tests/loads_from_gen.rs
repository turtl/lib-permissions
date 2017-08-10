extern crate lib_permissions;

#[cfg(test)]
mod tests {
    use ::lib_permissions::Role;

    #[test]
    fn get_perms() {
        Role::all_roles();
    }
}

