# finl-charsub

## Purpose

This is a Rust library for doing character replacement on strings. A standard 
character substitution definition file have a string of characters for the input
substitution, followed by white space and then the replacement text. For example,
the standard TeX input conventions can be reproduced using:
```
`	‘
``	“
'	’
''	”
--	–
---	—
!`	¡
?`	¿
~	\u{a0}
```
Escaping is handled following Rust conventions. If white space is to be used in the 
source or replacement text, it should be entered using escape codes (so `\u{20}`) to
get a space. I wouldn't recommend that, however.

Another example showing Sylvio Levy's scheme for input of classical Greek with ASCII
input:¹
```
a       α
'a      ά
`a      ὰ
~a      ᾶ
>a      ἀ
>'a     ἄ
>`a     ἂ
>~a     ἆ
<a      ἁ
<'a     ἅ
<`a     ἃ
<~a     ἃ
α|      \u{1fb3}
'a|     \u{1fb4}
`a|     \u{1fb2}
~a|     \u{1fb7}
>a|     \u{1f80}
>'a|    \u{1f84}
>`a|    \u{1f82}
>~a|    \u{1f86}
<a|     \u{1f81}
<'a|    \u{1f85}
<`a|    \u{1f83}
<~a|    \u{1f87}
   ⋮
   ⋮
;       \u{387}
''      ‘
?       \u{37e}
((      «
))      »
```

This is not meant as a replacement for a shaping library, but as a supplement to it.
In finl, charsubs will be applied before the shaping library is called but after all 
formatting commands

1. http://tug2.tug.org/TUGboat/tb09-1/tb20levy.pdf
