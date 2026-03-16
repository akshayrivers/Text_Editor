// Integers:
// 1 2 3 4 5 6 7 8 9 0, 100 200 300 400 500 1>2 1+1=2
// Floats:
// 1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 0.0
// Scientific Notation:
// 1e10, 20e50, 10.3e5,
// Visual Separators:
// 1_00, 1_000_1, 1_000_000_000
// Literals:
// 0x1, 0X2, 0b1, 0B0, 0X10F, 0o1

// Invalid Integers:
// 1a 2b 3c 4d 5e 6f 7g 8h 9i 0j, 100a200b300c400d500, u32, i8, f64, 1,2,3,4,5
// Invalid Floats:
// 1.1.2, 2.2.3, 3.3.4, 4.4.5, 5.5.6, 6.6.7, 7.7.8, 8.8.9, 9.9.0, 0.0.1
// Invalid Scientific Notation:
// 1e, e3, e, 1e2e, 5.8e10.1
// Invalid Visual Separators:
// _100_1, 100_, 1_00_, _
// Invalid Literals:
// 0b102 0x1G, 1o108, 0xxx

// Valid/Should be highlighted:
// '1' 'a' 'b' '👍' '\x1b', 'notacharacter' '\'', '\\' '1''2''3' '1'notchar'2'
// Invalid/ should not be highlighted:
// "a", 'b c'

// Valid/Should be highlighted as char:
// '1' 'a' 'b' '👍' '\x1b', 'notacharacter' '\'', '\\' '1''2''3' '1'notchar'2'
// Should be highlighted as lifetime specifier:
// 'a 'this_is_cool <'abc> '123
// Invalid/ should not be (fully) highlighted:
// "a", b' '   'invalid-specifier

/* This is a regular old ML comment
which goes on

until
there 👇
*/
struct foo; /* ml comments do not have to span multiple lines */
struct bar;
struct baz; /* or they start in the middle of a line


            and end in the middle of a line*/
struct f00;

/* they can contain things which should be ignored
- keywords like struct
- single line comments: //
- char definition: '
*/

/* and even worse: There are nested comments:
    /* which start in the middle  and end in the middle of an existing ML comment
    */
    and once they end, the original comment is still there.
*/

/* you need to highlight this correctly: /*/*// /**//**///*/*/*/*/*/*/**/*/*/*/*/*/*/*/*/*/
struct not_part_of_comment; /* part of a comment */

fn main() {
    let regular_str = "Hello, World!";
    let str_with_escape = "Hello \"Hello\" Hello!";
    let str_with_escape = "\"Hello\"\" Hello!";
    let str_with_escape = "\"Hello\"";
    let empty = "";
    let ml_string = "Hello, World!";
    let nested_1 = "Hello there!/* this is not actually a ML comment. It looks like one, but it's part of  a string. */    ";
    /* This is a ML commentIt looks like a multi line string ends here: "but that is not true, it's just a quote within an ML comment. */
}
