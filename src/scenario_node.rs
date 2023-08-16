//! group
//!   scene(同じ画層のまとまり:
//!         bgimg, bgcol, crop, lbl(ラベル名), lblref(ラベル名参照))
//!     page(クリック単位)
//!       mat(テキストの背景: col, pos, dim, r, a, v(縦書き), lbl, lblref, txt)
//!       ovimg(オーバーレイイメージ: path(画像), pos, a)
//!     pmat(クリック単位，matがひとつのみの場合，属性はmatと同じ)
//!
//! -> enum Item の候補は group / scene / page / mat / ovimg / pmat
//!    親子関係は
//!    1. group(管理の単位，実体なし)
//!      2. scene(同一背景の単位)
//!        3.1 page(クリックの単位，実体なし)
//!          3.1.1 mat
//!          3.1.2 ovimg
//!        3.2. pmat(pageの特殊形, matと等価)

use std::cell::{RefCell,Cell};
use std::rc::{Rc, Weak};
use std::fmt;

#[derive(Debug)]
pub struct ScenarioNode {
    pub value   : RefCell<Item>,
    pub bt      : Cell<BranchType>,
    pub parent  : RefCell<Weak<ScenarioNode>>, // Cellはコピー/置き換えになっちゃうのでRefCell
    pub child   : RefCell<Option<Rc<ScenarioNode>>>,
    pub neighbor: RefCell<Option<Rc<ScenarioNode>>>,
    pub id      : Cell<i32>,
}
impl Default for ScenarioNode{
    fn default() -> Self{
        ScenarioNode{
            value   : RefCell::new(Item::Page),
            bt      : Cell::new(BranchType::Child),
            parent  : RefCell::new(Weak::new()),
            child   : RefCell::new(None),
            neighbor: RefCell::new(None),
            id      : Cell::new(0),
        }
    }
}
// ScenarioNode ////////////////////////////////////////////////////
fn dump_mat(m: &Mat) -> String{
    let mut s= String::new();
    s+= &("M(".to_owned() +
          "c:" +
          &m.col.r.to_string() + "," +
          &m.col.g.to_string() + "," +
          &m.col.b.to_string() + "," +
          " p:" +
          &m.pos.x.to_string() + "," +
          &m.pos.y.to_string() + "," +
          " d:" +
          &m.dim.w.to_string() + "," +
          &m.dim.h.to_string() + "," +
          " r:" + &m.r.to_string() + "," +
          " a:" + &m.a.to_string() + "," +
          " " + &m.name + "),");
    if let Some(a)= &m.src   { s+= &("s".to_owned() + a); }
    if let Some(a)= &m.lbl   { s+= &("l".to_owned() + a); }
    if let Some(a)= &m.lblref{ s+= &("lr".to_owned() + a); }
    s
}
impl fmt::Display for ScenarioNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s= String::from( self.id.get().to_string() );
        match &(*self.value.borrow()){
            Item::Group    => s+= "G,",
            Item::Scene(c) => {
                s+= "S,";
                if let Some(a)= &c.bgimg{ s+= &("b[".to_owned() + a + "]"); }
                s+= &("c:".to_owned() +
                      &c.bgcol.r.to_string() + "," +
                      &c.bgcol.g.to_string() + "," +
                      &c.bgcol.b.to_string() + ",");
                if let Some(i)= &c.crop{
                    s+= "ci:";
                    s+= &(i.pos.x.to_string() + "," + &i.pos.y.to_string());
                    s+= &(i.dim.w.to_string() + "," + &i.dim.h.to_string());
                }
                if let Some(l)= &c.lbl   { s+= &("l".to_owned() + l); }
                if let Some(l)= &c.lblref{ s+= &("lr".to_owned() + l); }
            },
            Item::Page     => s+= "P,",
            Item::Mat(m)   => s+= &dump_mat(m),
            Item::Ovimg(o) => s+= &("O([".to_owned() + &o.path + "]," +
                                    "p:" +
                                    &o.pos.x.to_string() + "," +
                                    &o.pos.y.to_string() + "),"),
            Item::Pmat(m)  => s+= &dump_mat(m),
        }
        match self.bt.get(){
            BranchType::Child => s+= "b:c,",
            _                 => s+= "b:n,",
        }
        s+= "p:";
        if let Some(p) = &self.parent.borrow().clone().upgrade(){
            match &(*p.value.borrow()){
                Item::Group    => s+= "G",
                Item::Scene(_c)=> s+= "S",
                Item::Page     => s+= "P",
                Item::Mat(_m)  => s+= "M",
                Item::Ovimg(_o)=> s+= "O",
                Item::Pmat(_m) => s+= "pm",
            }
        }
        write!(f, "{}", s)
    }
}
impl ScenarioNode {
    pub fn set_value(&self, v: Item){
        *self.value.borrow_mut()= v;
    }
    pub fn set_bt(&self, bt: BranchType){
        self.bt.set(bt);
    }
    pub fn set_parent(&self, p: Weak<ScenarioNode>){
        *self.parent.borrow_mut()= p;
    }
    pub fn set_child(&self, c: Rc<ScenarioNode>){
        *self.child.borrow_mut()= Some(c);
    }
    pub fn set_neighbor(&self, n: Rc<ScenarioNode>){
        *self.neighbor.borrow_mut()= Some(n);
    }
    pub fn unset_neighbor(&self){
        *self.neighbor.borrow_mut()= None;
    }
    pub fn new() -> ScenarioNode{
        ScenarioNode{
            value   : RefCell::new(Item::Page),
            bt      : Cell::new(BranchType::Child),
            parent  : RefCell::new(Weak::new()),
            child   : RefCell::new(None),
            neighbor: RefCell::new(None),
            id      : Cell::new(0),
        }
    }
    pub fn remove(&self){
        let self_p= (*self.parent.borrow_mut()).upgrade();

        if self_p.is_some() { // parentあり -> root以外
            let self_p= self_p.unwrap().clone();
            let mut self_p_cn; // child or neighbor
            if self.bt == BranchType::Child.into() {
                self_p_cn= self_p.child.borrow_mut();
            } else {
                self_p_cn= self_p.neighbor.borrow_mut();
            }
            if let Some(self_n) = (*self.neighbor.borrow_mut()).as_ref(){
                *self_p_cn= Some(self_n.clone());
            } else {
                *self_p_cn= None;
            }
            if let Some(self_n) = (*self.neighbor.borrow_mut()).as_ref() {
                self_n.set_bt( self.bt.get() );
                self_n.set_parent(self.parent.clone().take());
            }
        } else { // rootの場合
            if let Some(self_n) = (*self.neighbor.borrow_mut()).as_ref() {
                self_n.set_bt(BranchType::Child);
                self_n.set_parent(Weak::new());
            }
        }
    }
    pub fn dump (&self, depth: usize){
        println!("{}{}", " ".repeat(depth), self);
        if let Some(c) = (*self.child.borrow_mut()).as_ref(){
            c.dump(depth + 2);
        }
        if let Some(n) = (*self.neighbor.borrow_mut()).as_ref(){
            n.dump(depth);
        }
    }
    // mv_to_parent ////////////////////////////////////////
    /// make B a child/neighbor of A's parent
    pub fn mv_to_parent(a: Rc::<ScenarioNode>, b: Rc<ScenarioNode>){
        if Rc::ptr_eq(&a, &b){ return; }

        // 1. remove B
        b.remove();
        // 2. set the parent of A to B,
        //   and the branch type of B to A's bt,
        //   and the branch tyep of A to neighbor
        b.set_bt(a.bt.get());
        a.set_bt(BranchType::Neighbor);
        // 3. set the neighbor of B to A
        b.set_neighbor(a.clone());

        if let Some(a_p) = (*a.parent.borrow_mut()).upgrade(){
            // 4. if exists, the neighbor of A's parent to B
            a_p.set_neighbor( b.clone() );
            // 5. if exists, set the parent of B to A's parent or empty.
            b.set_parent(Rc::downgrade(&a_p));
        } else {
            b.set_parent(Weak::new());
        }
        // 6. set the parent of A to B
        a.set_parent(Rc::downgrade(&b));

    }
    // mv_to_child /////////////////////////////////////////
    /// make B a child node of A
    pub fn mv_to_child(a: Rc::<ScenarioNode>, b: Rc<ScenarioNode>){
        if Rc::ptr_eq(&a, &b){ return; }

        // 1. remove B
        b.remove();
        // 2. set the parent of B to A, and the branch type of B to child
        b.set_parent(Rc::downgrade(&a));
        b.set_bt(BranchType::Child);
        // 3. set the neighbor of B to {the child of A or None},
        if let Some(a_c) = (*a.child.borrow_mut()).as_ref(){
            a_c.set_bt(BranchType::Neighbor);
            b.set_neighbor(a_c.clone());
        } else {
            b.unset_neighbor();
        }
        // 4. if exists, set the parent of child of A
        //    (in this timing, already B's neighbor) to B
        if let Some(b_n) = (*b.neighbor.borrow_mut()).as_ref(){
            b_n.set_parent(Rc::downgrade(&b));
        }
        // 5. set the child of A to B
        a.set_child(b.clone());
    }
    // mv_to_neighbor //////////////////////////////////////
    /// make B a neighbor of A
    pub fn mv_to_neighbor(a: Rc::<ScenarioNode>, b: Rc<ScenarioNode>){
        if Rc::ptr_eq(&a, &b){ return; }

        // 1. remove B
        b.remove();
        // 2. set the parent of B to A, and the branch type of B to neighbor
        b.set_parent(Rc::downgrade(&a));
        b.set_bt(BranchType::Neighbor);
        // 3. set the neighbor of B to {the neighbor of A or None},
        if let Some(a_n) = (*a.neighbor.borrow_mut()).as_ref(){
            a_n.set_bt(BranchType::Neighbor);
            b.set_neighbor(a_n.clone());
        } else {
            b.unset_neighbor();
        }
        // 4. if exists, set the parent of neighbor of A
        //    (in this timing, already B's neighbor) to B
        if let Some(b_n) = (*b.neighbor.borrow_mut()).as_ref(){
            b_n.set_parent(Rc::downgrade(&b));
        }
        // 5. set the neighbor of A to B
        a.set_neighbor(b.clone());
    }
}
// BranchType //////////////////////////////////////////////
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BranchType{ Child, Neighbor, }

// Item ////////////////////////////////////////////////////
#[derive(Debug)]
pub enum Item{
    Group,
    Scene(Scene),
    Page,
    Mat(Mat),
    Ovimg(Ovimg),
    Pmat(Mat)
}
// Color ///////////////////////////////////////////////////
#[derive(Debug)]
pub struct Color {
    pub r : u32,
    pub g : u32,
    pub b : u32,
}
// Position ////////////////////////////////////////////////
#[derive(Debug)]
pub struct Position {
    pub x : usize,
    pub y : usize,
}
// Dimension ///////////////////////////////////////////////
#[derive(Debug)]
pub struct Dimension {
    pub w : usize,
    pub h : usize,
}
// Ovimg ///////////////////////////////////////////////////
#[derive(Debug)]
pub struct Ovimg {
    pub path  : String,
    pub pos   : Position,
    pub a     : u8,
}
// CropInfo ////////////////////////////////////////////////
#[derive(Debug)]
pub struct CropInfo {
    pub pos : Position,
    pub dim : Dimension
}
// Scene ///////////////////////////////////////////////////
#[derive(Debug)]
pub struct Scene {
    pub bgimg : Option<String>,
    pub bgcol : Color,
    pub crop  : Option<CropInfo>,
    pub lbl   : Option<String>,
    pub lblref: Option<String>,
}
// Mat /////////////////////////////////////////////////////
#[derive(Debug)]
pub struct Mat {
    pub col   : Color,
    pub pos   : Position,
    pub dim   : Dimension,
    pub r     : usize,
    pub a     : u8,
    pub src   : Option<String>,
    pub lbl   : Option<String>,
    pub lblref: Option<String>,
    pub name  : String, // this field is only for debug
}
impl Mat {
    fn dump(&self) {
        println!{"    col= {:?}", self.col};
        println!{"    pos= {:?}", self.pos};
        println!{"    dim= {:?}", self.dim};
        println!{"    r= {}, a= {}", self.r, self.a};
        print_opt_str(&self.src,    String::from("    src"));
        print_opt_str(&self.lbl,    String::from("    lbl"));
        print_opt_str(&self.lblref, String::from("    lblref"));
    }
}
// print_opt_str ///////////////////////////////////////////
fn print_opt_str(a: &Option<String>, prefix: String){
    if let Some(s) = a {
        println!("{}= {}", prefix, s); }
    else {
        println!("{}= none", prefix) }
}

// debug
// impl Drop for ScenarioNode {
//     fn drop(&mut self) {
//         println!("> Dropping {}", self.id.get());
//     }
// }
