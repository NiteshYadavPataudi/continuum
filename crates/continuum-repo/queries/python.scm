; Python tree-sitter queries for symbol extraction

(function_definition
  name: (identifier) @name
  (#set! kind "fn"))

(class_definition
  name: (identifier) @name
  (#set! kind "class"))

(async_function_definition
  name: (identifier) @name
  (#set! kind "fn"))

(async_statement
  body: (function_definition
          name: (identifier) @name)
  (#set! kind "fn"))
