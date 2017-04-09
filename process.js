(function() {
"use strict";

var roles = null;
var permissions = null;
var role_permissions = null;

var process = function(permdata) {
	function to_kv(arr) {
		var obj = {};
		arr.forEach(function(item) {
			var key = item.replace(/-/g, '_');
			obj[key] = item;
		});
		return obj;
	};

	roles = to_kv(permdata.roles);
	permissions = to_kv(permdata.permissions);
	role_permissions = {};

	var copy = {};
	Object.keys(permdata.role_permissions).forEach(function(role) {
		var spec = permdata.role_permissions[role];
		if(!role_permissions[role]) role_permissions[role] = [];
		var perms = role_permissions[role];
		if(spec.copy) {
			// handle copies later
			copy[role] = spec;
			return;
		}
		if(spec.all_but) {
			permdata.permissions.forEach(function(role) {
				if(spec.all_but.indexOf(role) >= 0) return;
				perms.push(role);
			});
		}
		(spec.perms || []).forEach(function(perm) {
			perms.push(perm);
		});
	});
	Object.keys(copy).forEach(function(role) {
		var spec = copy[role];
		var perms = role_permissions[role];
		if(!spec.copy) return;
		role_permissions[spec.copy].forEach(function(perm) {
			perms.push(perm);
		});
		(spec.perms || []).forEach(function(perm) {
			perms.push(perm);
		});
	});

	return {
		roles: roles,
		permissions: permissions,
		role_permissions: role_permissions,
	};
};

var can = function(role, permission) {
	var role_perms = role_permissions[role];
	if(permissions && permissions.indexOf(permission) >= 0) {
		return true;
	}
	return false;
};

this.Permissions = {
	process: process,
	can: can
};

}).call(typeof(exports) === 'undefined' ? window : exports);

