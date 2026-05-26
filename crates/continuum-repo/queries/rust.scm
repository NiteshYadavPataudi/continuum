; Rust tree-sitter queries for symbol extraction
; Matches functions, structs, enums, traits, modules, type aliases, impl blocks

(function_item
  name: (identifier) @name
  (#set! kind "fn"))

(struct_item
  name: (type_identifier) @name
  (#set! kind "struct"))

(enum_item
  name: (type_identifier) @name
  (#set! kind "enum"))

(trait_item
  name: (type_identifier) @name
  (#set! kind "trait"))

(mod_item
  name: (identifier) @name
  (#set! kind "mod"))

(type_item
  name: (type_identifier) @name
  (#set! kind "type"))

(impl_item
  trait: (trait_impl
           path: (identifier) @name)
  (#set! kind "impl"))
