"use strict";

var yaml = require('js-yaml');
var fs = require('fs');
var perms = require('./process').Permissions;

var permdata = require('./permissions.json');

module.exports = perms.process(permdata);

