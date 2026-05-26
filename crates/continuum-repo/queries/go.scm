; Go tree-sitter queries for symbol extraction

(function_declaration
  name: (identifier) @name
  (#set! kind "fn"))

(method_declaration
  name: (field_identifier) @name
  (#set! kind "fn"))

(struct_type
  name: (type_identifier) @name
  (#set! kind "struct"))

(interface_type
  name: (type_identifier) @name
  (#set! kind "interface"))

(type_declaration
  (type_spec
    name: (type_identifier) @name)
  (#set! kind "type"))
