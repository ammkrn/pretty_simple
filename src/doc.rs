#![allow(unused_parens)]
use std::sync::Arc;

use InnerDoc::*;
use crate::parenable::Parenable;


/*
If you're pre-calculating in the constructors, there's no need
to differentiate between dist_newline and dist_first_newline
since the distinction was made on construction in the 'concat' 
element's constructor.
*/

#[derive(Debug, Clone, Copy)]
pub struct RenderInfo {
    flatmode : bool,
    nest : usize,
    dist_next_newline : usize,
    line_width : usize,
}

impl RenderInfo {
    pub fn new(flatmode : bool, nest : usize, dist_next_newline : usize, line_width : usize) -> Self {
        RenderInfo {
            flatmode,
            nest,
            dist_next_newline,
            line_width
        }
    }
}


/*
Fundamentally, the leaf constructors `Text` and `Newline`
form the actual text of what you want to render. Everything else
is either a unary node (like Group and Nest) or binary node (Concat)
that controls the layout and directs execution of the `render`
function.
*/

/*
Documents are represented internally by a left-spined
tree of other smaller documents (some of which carry 
your text/data to be printed)
The tree you end up with is left-spined

             C
           /   \
         C     d4
       /  \
     C    d3
   /  \
  d1  d2
*/


// Having tree/daglike recursive structures in rust requires this
// kind of indirection. Since Doc implements `AsRef<Target = InnerDoc>`, 
// all of the methods defined on `InnerDoc` can be accessed via a `Doc`.
// The only difference you're likely to experience is that when pattern 
// matching, you'll need to use `match d.as_ref()` instead of `match d`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc(Arc<InnerDoc>);

// Standard wadler-style pretty printer items. The only difference
// between Newline and NewlineZero is that when printing in flatmode,
// a Newline will be rendered as a space, and a NewlineZero (for zero-width)
// will not insert a space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InnerDoc {
    Nil,
    Newline,
    NewlineZero,
    Text { 
        s : String, 
        len : usize
    },
    Concat { 
        lhs : Doc, 
        rhs : Doc, 
        has_newline : bool, 
        dist_newline : usize,
        flat_len : usize,
    },
    Nest { 
        nest : usize, 
        doc : Doc, 
        has_newline : bool, 
        dist_newline : usize,
        flat_len : usize,
    },
    Group { 
        doc : Doc, 
        has_newline : bool, 
        dist_newline : usize,
        flat_len : usize,
    }
}


impl Doc {
    fn get_has_newline(&self) -> bool {
        match self.as_ref() {
            Nil                        => false,
            Newline | NewlineZero      => true,
            Concat { has_newline, .. } => *has_newline,
            Nest   { has_newline, .. } => *has_newline,
            Group  { has_newline, .. } => *has_newline,
            Text   { .. }              => false,
        }
    }

    pub fn get_dist_newline(&self) -> usize {
        match self.as_ref() {
            Concat { dist_newline, .. } => *dist_newline,
            Nest   { dist_newline, .. } => *dist_newline,
            Group  { dist_newline, .. } => *dist_newline,
            Text   { len, .. }          => *len,
            _                           => 0
        }
    }

    pub fn get_flat_len(&self) -> usize {
        match self.as_ref() {
            Concat { flat_len, .. }     => *flat_len,
            Nest   { flat_len, .. }     => *flat_len,
            Group  { flat_len, .. }     => *flat_len,
            Text   { len, .. }          => *len,
            Newline                     => 1,
            _                           => 0
        }
    }
 
    pub fn nil() -> Self {
        Doc::from(Nil)
    }

    pub fn text(s : String) -> Self {
        let len = s.len();
        Doc::from(Text { 
            s,
            len
        })
   }

    pub fn nest(&self, n : usize) -> Self {
        Doc::from(Nest {
            nest : n,
            doc : self.clone(),
            has_newline : self.get_has_newline(),
            dist_newline : self.get_dist_newline(),
            flat_len : self.get_flat_len(),
        })
   }

    pub fn concat(&self, other : impl Into<Doc>) -> Self {
        let other : Doc = other.into();
        Doc::from(Concat {
             lhs : self.clone(),
             rhs : other.clone(),
             has_newline : (self.get_has_newline()) || (other.get_has_newline()),
             dist_newline : if self.get_has_newline() {
                 self.get_dist_newline()
             } else {
                 self.get_dist_newline() + other.get_dist_newline()
             },
             flat_len : self.get_flat_len() + other.get_flat_len(),
         })
    }

    // make (d1, newline, d2)
    pub fn concat_newline(self, other : impl Into<Doc>) -> Doc {
        self.concat(Newline)
            .concat(other)
    }

    // make (d1, space, d2)
    pub fn concat_space(self, other : impl Into<Doc>) -> Doc {
        self.concat(format!(" "))
            .concat(other)
    }

    pub fn group(&self) -> Self {
        Doc::from(Group {
            doc : self.clone(),
            has_newline : self.get_has_newline(),
            dist_newline : self.get_dist_newline(),
            flat_len : self.get_flat_len(),
        })
    }


    pub fn line() -> Self {
        Doc::from(Newline)
    }

    pub fn newline() -> Self {
        Doc::from(Newline)
    }

    pub fn newline_zero() -> Self {
        Doc::from(NewlineZero)
    }

    pub fn surround_paren(self) -> Self {
        Doc::from("(")
        .concat(self)
        .concat(")")
    }

    pub fn surround_curly(self) -> Self {
        Doc::from("{")
        .concat(self)
        .concat("}")
    }

    pub fn surround_square(self) -> Self {
        Doc::from("[")
        .concat(self)
        .concat("]")
    }

    // The stuff with RenderInfo is so we can easily make this
    // iterative instead of recursive.
    pub fn render(&self, line_width : usize) -> String {
        let mut todos = Vec::with_capacity(256);
        todos.push((self, RenderInfo::new(false, 0, 0, line_width)));

        let mut eol = line_width;
        let mut acc = String::new();

        while let Some((doc, info)) = todos.pop() {
            match doc.as_ref() {
                Nil => continue,
                Newline if info.flatmode => { acc.push_str(" "); },
                NewlineZero if info.flatmode => continue,
                Newline | NewlineZero => {
                    assert!(!info.flatmode);
                    acc.push_str("\n");
                    eol = (acc.len() + info.line_width);
                    for _ in 0..info.nest {
                        acc.push(' ');
                    }
                }
                Text { s, .. } => acc.push_str(s.as_str()),
                Concat { lhs, rhs, .. } => {
                    let lhs_dist_next_newline = if rhs.get_has_newline() {
                        rhs.get_dist_newline()
                    } else {
                        rhs.get_dist_newline() + info.dist_next_newline
                    };

                    let lhs_info = RenderInfo::new(info.flatmode, 
                                                   info.nest,
                                                   lhs_dist_next_newline,
                                                   info.line_width);
                    todos.push((rhs, info));
                    todos.push((lhs, lhs_info));
                },
                Nest { nest : spaces, doc : inner, .. } => {
                    let inner_info = RenderInfo::new(info.flatmode,
                                                     info.nest + spaces,
                                                     info.dist_next_newline,
                                                     info.line_width);
                    todos.push((inner, inner_info));
                },
                Group { doc : inner, .. } => {
                    let flat_bool = (info.flatmode || (acc.len() + inner.get_flat_len() + info.dist_next_newline <= eol));
                    let inner_info = RenderInfo::new(flat_bool, info.nest, info.dist_next_newline, info.line_width);
                    todos.push((inner, inner_info));
                },
           }
        }
        acc
    }
 

    pub fn as_parenable_max(self) -> Parenable {
        Parenable::new_max(self)
    }

    pub fn as_parenable(self, priority : usize) -> Parenable {
        Parenable::new(self, priority)
    }

}

// Take a list of documents and make a tree by concatenating them.
// IE turn [d1, d2, d3, d4] into :
//             C
//           /   \
//         C     d4
//       /  \
//     C    d3
//   /  \
//  d1  d2
pub fn sep(docs : &[Doc]) -> Doc {
    let mut as_iter = docs.into_iter().cloned();
    match as_iter.next() {
        None => Doc::nil(),
        Some(fst) => as_iter.fold(fst, |acc, next| acc.concat(next))
    }
}


/*
 turn an iterator [d1, d2, d3, d4] into

                             C
                           /   \
                         /      \
                        C        Group
                      /  \         |
                    /     \       C (\n, d4)
                  C       Group (C)
                /  \        |
              /     \      C (\n, d3)
            d1     Group (C)  
                     |
                    C (\n, d2)
*/
pub fn word_wrap_val<I>(mut s : I) -> Doc 
where I : Iterator<Item = Doc> + Clone {
    if let Some(hd) = s.next() {
        s.fold(hd, |acc, elem| acc.concat(Doc::line().concat(elem).group()))
    } else {
        Doc::nil()
    }
}


impl<T> From<T> for Doc 
where T : std::fmt::Display {
    fn from(t : T) -> Doc {
        Doc::text(format!("{}", t))
    }
}

impl std::convert::AsRef<InnerDoc> for Doc {
    fn as_ref(&self) -> &InnerDoc {
        match self {
            Doc(x) => x.as_ref()
        }
    }
}

impl From<InnerDoc> for Doc {
    fn from(t : InnerDoc) -> Doc {
        Doc(Arc::new(t))
    }
}

impl From<&InnerDoc> for Doc {
    fn from(t : &InnerDoc) -> Doc {
        Doc(Arc::new(t.clone()))
    }
}

