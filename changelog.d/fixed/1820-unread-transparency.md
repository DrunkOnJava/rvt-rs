- **An IFC surface style no longer states that a material is opaque when its
  transparency was not read (#1820).** A material whose colour was read but
  whose transparency was not used to be written with a transparency of 0,
  fully opaque, a value no file gave. That optional attribute is now left
  unset (`$`). No material of the measured models is read that way, so no
  export changes.
