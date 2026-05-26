(function_declaration name: (identifier) @fn.name) @fn.def
(class_declaration name: (type_identifier) @class.name) @class.def
(interface_declaration name: (type_identifier) @iface.name) @iface.def
(enum_declaration name: (identifier) @enum.name) @enum.def
(type_alias_declaration name: (type_identifier) @type.name) @type.def
(module name: (identifier) @mod.name) @mod.def
(export_statement declaration: (function_declaration name: (identifier) @fn.name)) @fn.def
