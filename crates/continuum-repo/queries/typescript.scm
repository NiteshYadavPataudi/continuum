; TypeScript / JavaScript tree-sitter queries for symbol extraction

(function_declaration
  name: (identifier) @name
  (#set! kind "fn"))

(generator_function_declaration
  name: (identifier) @name
  (#set! kind "fn"))

(class_declaration
  name: (type_identifier) @name
  (#set! kind "class"))

(interface_declaration
  name: (type_identifier) @name
  (#set! kind "interface"))

(enum_declaration
  name: (type_identifier) @name
  (#set! kind "enum"))

(type_alias_declaration
  name: (type_identifier) @name
  (#set! kind "type"))

(module
  name: (identifier) @name
  (#set! kind "module"))

(arrow_function
  name: (identifier) @name
  (#set! kind "fn"))

(export_statement
  value: (function_declaration
           name: (identifier) @name) @name
  (#set! kind "fn"))
