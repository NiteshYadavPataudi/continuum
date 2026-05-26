(function_item name: (identifier) @fn.name) @fn.def
(struct_item name: (type_identifier) @struct.name) @struct.def
(enum_item name: (type_identifier) @enum.name) @enum.def
(trait_item name: (type_identifier) @trait.name) @trait.def
(impl_item trait: (type_identifier) @impl.target) @impl.def
(impl_item type: (type_identifier) @impl.ty) @impl.def
(mod_item name: (identifier) @mod.name) @mod.def
