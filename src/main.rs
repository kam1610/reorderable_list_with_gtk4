mod scenario_node;
mod scenario_node_object;
mod isv2_button;
mod operation_history;
mod scenario_item_drag_object;

use std::cell::Cell;

use crate::scenario_node_object::ScenarioNodeObject;
use crate::scenario_node_object::adj_seq;
use crate::scenario_node_object::add_neighbor;
use crate::scenario_node_object::add_child;
use crate::scenario_node_object::remove_node;
use crate::scenario_item_drag_object::ScenarioItemDragObject;
use crate::scenario_node::ScenarioNode;
use crate::scenario_node::BranchType;
use crate::operation_history::Operation;
use crate::operation_history::OperationHistory;
use crate::operation_history::OperationHistoryItem;
use crate::operation_history::TreeManipulationHandle;

use crate::isv2_button::Isv2Button;

use gdk4::Display;
use gtk::{
    gio, glib, Application, ApplicationWindow, Label, ListView, PolicyType,
    ScrolledWindow, SignalListItemFactory, SingleSelection,
    TreeExpander, TreeListModel, TreeListRow, glib::object::Object, gio::ListModel,
    CssProvider,
    Orientation, Box, Button,
    Widget
};
use gtk::{prelude::*, ListItem, DragSource};

mod integer_object;
use gdk4::ContentProvider;
use gdk4::DragAction;
use gtk::DropTarget;
use glib::value::*;

use std::sync::atomic::{AtomicI32, Ordering};
use std::rc::Rc;

// use xmltree::Element;
// use xmltree::EmitterConfig;
// use std::fs::File;
// use gtk::{DrawingArea, TextView, Switch};
// use gtk::cairo::{FontSlant, FontWeight};
// use pangocairo;
// use pango::FontDescription;

const APP_ID: &str = "org.gtk_rs.ImageScenarioView2";

// get_seq /////////////////////////////////////////////////
fn get_seq() -> i32 {
    static COUNT: AtomicI32 = AtomicI32::new(1000);
    COUNT.fetch_add(1, Ordering::SeqCst)
}
// load_css ////////////////////////////////////////////////
fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("style.css"));

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
// main ////////////////////////////////////////////////////
fn main() -> glib::ExitCode {
    println!("--------");
    ////////////////////////////////////////////////////////
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);
    app.run()
}
// append_neighbors ////////////////////////////////////////
fn append_neighbors(model: &gio::ListStore, sn: Rc<ScenarioNode>, seq: i32){
    let obj= ScenarioNodeObject::new_from(sn.clone());
    obj.set_seq(seq);
    model.append( &obj );
    if let Some(nbr) = (*sn.neighbor.borrow_mut()).as_ref(){
        append_neighbors(model, nbr.clone(), seq+1);
    }
}
// my_creator //////////////////////////////////////////////
fn my_creator(obj: &Object) -> Option<ListModel>{

    let sn= obj.downcast_ref::<ScenarioNodeObject>().expect("ScenarioNodeObject is expected");
    if let Some(c) = (*sn.get_node().child.borrow_mut()).as_ref() {
        let model = gio::ListStore::new(ScenarioNodeObject::static_type());

        append_neighbors(&model, c.clone(), 0);

        Some(model.into())
    } else {
        None
    }

}

// expander_to_store ///////////////////////////////////////

// TODO: 空になった直後のexpanderを開こうとするとクラッシュする
fn expander_to_store(e: &TreeExpander, depth: u32) -> gio::ListStore {
    if depth == 0 {
        e.parent().unwrap()
            .parent().and_downcast::<ListView>().expect("ListView is expected")
            .model().unwrap() // SelectionModel
            .downcast::<SingleSelection>().expect("SingleSelection")
            .model().unwrap() // TreeListModel
            .downcast::<TreeListModel>().expect("TreeListModel")
            .model()          // ListModel
            .downcast::<gio::ListStore>().expect("ListStore")
    } else {
        e.list_row().unwrap()
            .parent().unwrap()   // depthが0の場合はこのunwrapが失敗するので，parent...経由ででListViewを取得する
            .children().unwrap() // ListModel
            .downcast::<gio::ListStore>().expect("ListStore")
    }
}

// label_drop_remove_style //////////////////////////////////////
fn label_drop_remove_style(w: Widget, u: bool, l: bool) {
    if u { w.add_css_class   ("indicate_upper"); }
    else { w.remove_css_class("indicate_upper"); }

    if l { w.add_css_class   ("indicate_lower"); }
    else { w.remove_css_class("indicate_lower"); }
}

// Operation when a label is dropped to Label / Expander
//
// | drop       | bt of    | drop area                         |
// | target     | dest     |------------------+----------------|
// |            |          | upper half       | lower half     |
// |------------+----------+------------------+----------------|
// | Label      | child    | parent child     | dest child     |
// |            | neighbor | parent neighbor  | dest child     |
// |------------+----------+------------------+----------------|
// | Expander   | child    | parent child     | dest neighbor  |
// |            | neighbor | parent neighbor  | dest neighbor  |
//
// 基本は上記で作成，
// Item種別(Group, Scene, Page, Mat, Ovimg, Pmat)間の関係で
// ダメな場合は，child/neighborを入れ替えて試行

// expander_to_dest_member /////////////////////////////////
fn expander_to_dest_member2(e: &TreeExpander, root_store: gio::ListStore)
                            -> TreeManipulationHandle{
    let dest_row= e.list_row().unwrap();
    let dest_sno= dest_row
        .item().and_downcast::<ScenarioNodeObject>().expect("sno is expd");
    let dest_depth= dest_row.depth();
    let dest_store= expander_to_store(e, dest_depth);

    let dest_parent_row   = row_to_parent_row(&dest_row);
    let dest_parent_store = row_to_parent_store(&dest_parent_row, &root_store);
    let dest_parent_sno   = get_parent_sno(&dest_sno, &dest_parent_row, &dest_store);

    TreeManipulationHandle{
        bt           : dest_sno.get_bt().into(),
        row          : Some(dest_row.clone().into()),
        sno          : Some(dest_sno.clone().into()),
        store        : Some(dest_store.clone().into()),
        depth        : Cell::new(dest_row.depth()),
        size         : Cell::new(dest_store.n_items()),
        parent_row   : Some(dest_parent_row.clone().into()),
        parent_sno   : Some(dest_parent_sno.into()),
        parent_store : Some(dest_parent_store.into()),
    }
}
// src_value_to_src_member /////////////////////////////////
fn src_value_to_src_member2(v: &Value) ->
    (TreeManipulationHandle, gio::ListStore, Rc<OperationHistory>){

        let drag_obj = v.get::<ScenarioItemDragObject>().expect("scn itm drag obj is expd");

        let src_row = drag_obj
            .get_list_item()
            .item().and_downcast::<TreeListRow>().expect("tlrow is expected");
        let src_sno = src_row
            .item().and_downcast::<ScenarioNodeObject>().expect("sno is expected");
        let src_depth = src_row.depth();
        let src_store = expander_to_store(&drag_obj
                                          .get_list_item()
                                          .child().expect("child")
                                          .downcast::<TreeExpander>().expect("TreeExpander"),
                                          src_depth);

        let src_parent_row   = row_to_parent_row(&src_row);
        let src_parent_store = row_to_parent_store(&src_parent_row, &drag_obj.get_root_store());
        let src_parent_sno   = get_parent_sno(&src_sno, &src_parent_row, &src_store);

        let hdl = TreeManipulationHandle{
            bt           : src_sno.get_bt().into(),
            row          : Some(src_row.clone().into()),
            sno          : Some(src_sno.clone().into()),
            store        : Some(src_store.clone().into()),
            depth        : Cell::new(src_depth),
            size         : Cell::new(src_store.n_items()),
            parent_row   : Some(src_parent_row.clone().into()),
            parent_sno   : Some(src_parent_sno.into()),
            parent_store : Some(src_parent_store.into()),
        };
        (hdl, drag_obj.get_root_store(), drag_obj.get_history())
}
// add_node_to_empty_store /////////////////////////////
fn add_node_to_empty_store(a: Isv2Button, sno: &ScenarioNodeObject) {
    a.get_store().insert( 0, sno );
}
// isv2button_to_dest_member ///////////////////////////////
fn row_to_parent_store(row: &TreeListRow, root: &gio::ListStore) -> gio::ListStore {
    if row.depth() > 0 {
        row.parent().unwrap()
            .children().unwrap() // ListModel
            .downcast::<gio::ListStore>().expect("ListStore")
    } else {
        root.clone()
    }
}
fn row_to_parent_row(r: &TreeListRow) -> TreeListRow {
    if r.depth() == 0 { // root-root
        r.clone()
    } else { // (Child &&, depth > 0) or neighbor
        r.parent().unwrap()
    }
}

fn get_parent_sno(sno: &ScenarioNodeObject,
                  parent_row: &TreeListRow,
                  store: &gio::ListStore) -> ScenarioNodeObject {
    if sno.get_bt() == BranchType::Child {
        parent_row.item().and_downcast::<ScenarioNodeObject>().expect("sno is expd").clone()
    } else {
        let parent_item = store.item( sno.get_seq() as u32 - 1).unwrap();
        parent_item
            .downcast_ref::<ScenarioNodeObject>().unwrap().clone()
    }
}
fn isv2button_to_dest_member4(b: &Isv2Button) ->
    Result<TreeManipulationHandle, &'static str> {

        if b.get_selection().selected_item().is_none() {
            return Err("not selected"); }

        let root_store= b.get_selection() // selection is a member of Isv2Button
            .model().unwrap() // TreeListModel
            .downcast::<TreeListModel>().expect("TreeListModel")
            .model()          // ListModel
            .downcast::<gio::ListStore>().expect("ListStore");

        let obj               = b.get_selection().selected_item().unwrap();
        let dest_row          = obj.downcast_ref::<TreeListRow>().expect("TreeListRow is expected");
        let dest_sno          = dest_row.item().and_downcast::<ScenarioNodeObject>().expect("sno is expd");
        let dest_store        = row_to_parent_store(dest_row, &root_store);

        let dest_parent_row   = row_to_parent_row(dest_row);
        let dest_parent_store = row_to_parent_store(&dest_parent_row, &root_store);
        let dest_parent_sno   = get_parent_sno(&dest_sno, &dest_parent_row, &dest_store);

        let hdl = TreeManipulationHandle{
            bt           : dest_sno.get_bt().into(),
            row          : Some(dest_row.clone().into()),
            sno          : Some(dest_sno.into()),
            store        : Some(dest_store.clone().into()),
            depth        : Cell::new(dest_row.depth()),
            size         : Cell::new(dest_store.n_items()),
            parent_row   : Some(dest_parent_row.clone().into()),
            parent_sno   : Some(dest_parent_sno.into()),
            parent_store : Some(dest_parent_store.into()),
        };
        Ok( hdl )
    }
// detect_descendant ///////////////////////////////////////
fn detect_descendant(parent: &TreeListRow, child: &TreeListRow) -> bool {
    if parent == child {
        return true; }
    if child.parent().is_none() {
        return false; }
    return detect_descendant(parent, &child.parent().unwrap());
}

// expander_drop_function //////////////////////////////////
fn expander_drop_function(d: &DropTarget, v: &Value, _x: f64, y: f64) -> bool{
    // obtain src
    let (src_hdl, root_store, history) =
        src_value_to_src_member2(v);
    let src_row    = src_hdl.row.as_ref().unwrap();
    let src_sno    = src_hdl.sno.as_ref().unwrap();
    let src_store  = src_hdl.store.as_ref().unwrap();

    // obtain dest
    let dest_hdl =
        expander_to_dest_member2( &d.widget()
                                   .downcast::<TreeExpander>()
                                   .expect("expander is expected"),
                                   root_store);
    let dest_sno        = dest_hdl.sno.as_ref().unwrap();
    let dest_row        = dest_hdl.row.as_ref().unwrap();
    let dest_parent_sno = dest_hdl.parent_sno.as_ref().unwrap();
    let dest_store      = dest_hdl.store.as_ref().unwrap();

    // check: move to descendant -> ignore
    if detect_descendant(&src_row, &dest_row) {
        println!("moving to descendant is ignored");
        label_drop_remove_style( d.widget(), false, false );
        return false;
    }

    let new_node= ScenarioNodeObject::new_from( src_sno.get_node() );

    let mut h= OperationHistoryItem::default();

    if y < (d.widget().height()/2).into() { // upper half
        if dest_sno.get_bt() == BranchType::Child { // parent に mv_to_child
            if (*dest_sno.get_node().parent.borrow_mut()).upgrade().is_some() {
                h.ope = Operation::MvToParentChild.into();
                ScenarioNode::mv_to_child(dest_parent_sno.get_node(), new_node.get_node());
            } else {
                h.ope = Operation::MvToParent.into();
                ScenarioNode::mv_to_parent(dest_sno.get_node(), new_node.get_node());
            }
        } else { // parent に mv_to_neighbor
            h.ope = Operation::MvToParentNeighbor.into();
            ScenarioNode::mv_to_neighbor(dest_parent_sno.get_node(), new_node.get_node());
        }
        new_node.set_seq( dest_sno.get_seq() );
        adj_seq( &dest_store, dest_sno.get_seq(), 1 );
        dest_store.insert( (dest_sno.get_seq() as u32) - 1, &new_node ); // -1: because +1 at previouse adj_seq()
    } else { // lower-half -> dest に mv_to_neighbor
        h.ope= Operation::MvToDestNeighbor.into();
        ScenarioNode::mv_to_neighbor(dest_sno.get_node(), new_node.get_node());
        new_node.set_seq( dest_sno.get_seq() + 1 );
        adj_seq( &dest_store, dest_sno.get_seq() + 1, 1 );
        dest_store.insert( (dest_sno.get_seq() as u32) + 1, &new_node );
    }
    // remove src
    adj_seq(&src_store, src_sno.get_seq() + 1, -1);
    src_store.remove( src_sno.get_seq() as u32 );

    label_drop_remove_style( d.widget(), false, false );

    h.src     = src_hdl;
    h.dest    = dest_hdl;
    h.new_sno = Some(new_node.clone().into());
    history.push(h.clone());

    true
}
// label_drop_function /////////////////////////////////////
fn label_drop_function(d: &DropTarget, v: &Value, _x: f64, y: f64) -> bool{

    // obtain src
    let (src_hdl, root_store, history) =
        src_value_to_src_member2(v);
    let src_row    = src_hdl.row.as_ref().unwrap();
    let src_sno    = src_hdl.sno.as_ref().unwrap();
    let src_store  = src_hdl.store.as_ref().unwrap();

    // obtain dest
    let dest_hdl =
        expander_to_dest_member2( &d.widget()
                                   .parent().and_downcast::<TreeExpander>()
                                   .expect("expander is expected"),
                                   root_store);
    let dest_sno        = dest_hdl.sno.as_ref().unwrap();
    let dest_row        = dest_hdl.row.as_ref().unwrap();
    let dest_parent_sno = dest_hdl.parent_sno.as_ref().unwrap();
    let dest_store      = dest_hdl.store.as_ref().unwrap();

    // check: move to descendant -> ignore
    if detect_descendant(&src_row, &dest_row) {
        println!("moving to descendant is ignored");
        label_drop_remove_style( d.widget(), false, false );
        return false;
    }

    let new_node= ScenarioNodeObject::new_from( src_sno.get_node() );

    let mut h= OperationHistoryItem::default();

    if y < (d.widget().height()/2).into() { // upper-half
        if dest_sno.get_bt() == BranchType::Child { // parent に mv_to_child
            if (*dest_sno.get_node().parent.borrow_mut()).upgrade().is_some() {
                h.ope = Operation::MvToParentChild.into();
                ScenarioNode::mv_to_child(dest_parent_sno.get_node(), new_node.get_node());
            } else {
                h.ope = Operation::MvToParent.into();
                ScenarioNode::mv_to_parent(dest_sno.get_node(), new_node.get_node());
            }
        } else { // parent に mv_to_neighbor
            h.ope = Operation::MvToParentNeighbor.into();
            ScenarioNode::mv_to_neighbor(dest_parent_sno.get_node(), new_node.get_node());
        }
        new_node.set_seq( dest_sno.get_seq() );
        adj_seq( &dest_store, dest_sno.get_seq(), 1 );
        dest_store.insert( (dest_sno.get_seq() as u32) - 1, &new_node ); // -1: because +1 at previouse adj_seq()

        // remove src
        adj_seq(&src_store, src_sno.get_seq() + 1, -1);
        src_store.remove( src_sno.get_seq() as u32 );
    } else { // lower-half -> dest child
        h.ope= Operation::MvToDestChild.into();
        ScenarioNode::mv_to_child(dest_sno.get_node(), new_node.get_node());
        new_node.set_seq( 0 );
        if let Some(m) = dest_row.children(){
            let s= m.downcast::<gio::ListStore>().expect("ListStore");
            adj_seq( &s, 0, 1 );
            s.insert( 0, &new_node );
        }
        else {
            let dest_node= ScenarioNodeObject::new_from( dest_sno.get_node() );
            dest_node.set_seq( dest_sno.get_seq() );
            dest_store.remove( dest_sno.get_seq() as u32 );
            dest_store.insert( dest_sno.get_seq() as u32, &dest_node );
        }

        // remove src
        adj_seq(&src_store, src_sno.get_seq() + 1, -1);
        src_store.remove( src_sno.get_seq() as u32 );
    }

    label_drop_remove_style( d.widget(), false, false );

    h.src     = src_hdl;
    h.dest    = dest_hdl;
    h.new_sno = Some(new_node.clone().into());
    history.push(h.clone());

    true
}

// build_ui ////////////////////////////////////////////////
fn build_ui(app: &Application) {

    let model = gio::ListStore::new(ScenarioNodeObject::static_type());

    let o_node1   = ScenarioNodeObject::new_with_seq_id(0, 1  );
    let o_node2   = ScenarioNodeObject::new_with_seq_id(0, 2  ); o_node2.set_parent( o_node1.get_node() ); o_node2.set_bt(BranchType::Neighbor);
    let o_node3   = ScenarioNodeObject::new_with_seq_id(0, 3  ); o_node3.set_parent( o_node2.get_node() ); o_node3.set_bt(BranchType::Neighbor);
    let o_node31  = ScenarioNodeObject::new_with_seq_id(0, 31 ); o_node31.set_parent( o_node3.get_node() ); o_node31.set_bt(BranchType::Child);
    let o_node32  = ScenarioNodeObject::new_with_seq_id(0, 32 ); o_node32.set_parent( o_node31.get_node() ); o_node32.set_bt(BranchType::Neighbor);
    let o_node33  = ScenarioNodeObject::new_with_seq_id(0, 33 ); o_node33.set_parent( o_node32.get_node() ); o_node33.set_bt(BranchType::Neighbor);
    let o_node331 = ScenarioNodeObject::new_with_seq_id(0, 331); o_node331.set_parent( o_node33.get_node() ); o_node331.set_bt(BranchType::Child);
    let o_node332 = ScenarioNodeObject::new_with_seq_id(0, 332); o_node332.set_parent( o_node331.get_node() ); o_node332.set_bt(BranchType::Neighbor);
    let o_node333 = ScenarioNodeObject::new_with_seq_id(0, 333); o_node333.set_parent( o_node332.get_node() ); o_node333.set_bt(BranchType::Neighbor);
    let o_node34  = ScenarioNodeObject::new_with_seq_id(0, 34 ); o_node34.set_parent( o_node33.get_node() ); o_node34.set_bt(BranchType::Neighbor);
    let o_node35  = ScenarioNodeObject::new_with_seq_id(0, 35 ); o_node35.set_parent( o_node34.get_node() ); o_node35.set_bt(BranchType::Neighbor);
    let o_node36  = ScenarioNodeObject::new_with_seq_id(0, 36 ); o_node36.set_parent( o_node35.get_node() ); o_node36.set_bt(BranchType::Neighbor);
    let o_node361 = ScenarioNodeObject::new_with_seq_id(0, 361); o_node361.set_parent( o_node36.get_node() ); o_node361.set_bt(BranchType::Child);
    let o_node362 = ScenarioNodeObject::new_with_seq_id(0, 362); o_node362.set_parent( o_node361.get_node() ); o_node362.set_bt(BranchType::Neighbor);
    let o_node363 = ScenarioNodeObject::new_with_seq_id(0, 363); o_node363.set_parent( o_node362.get_node() ); o_node363.set_bt(BranchType::Neighbor);
    let o_node4   = ScenarioNodeObject::new_with_seq_id(0, 4  ); o_node4.set_parent( o_node3.get_node() ); o_node4.set_bt(BranchType::Neighbor);
    let o_node5   = ScenarioNodeObject::new_with_seq_id(0, 5  ); o_node5.set_parent( o_node4.get_node() ); o_node5.set_bt(BranchType::Neighbor);
    let o_node6   = ScenarioNodeObject::new_with_seq_id(0, 6  ); o_node6.set_parent( o_node5.get_node() ); o_node6.set_bt(BranchType::Neighbor);

    o_node1.set_neighbor( o_node2.get_node() );
    o_node2.set_neighbor( o_node3.get_node() );
    o_node3.set_neighbor( o_node4.get_node() );
    o_node4.set_neighbor( o_node5.get_node() );
    o_node5.set_neighbor( o_node6.get_node() );

    o_node3.set_child    ( o_node31.get_node() );
    o_node31.set_neighbor( o_node32.get_node() );
    o_node32.set_neighbor( o_node33.get_node() );

    o_node33.set_child( o_node331.get_node() );
    o_node331.set_neighbor( o_node332.get_node() );
    o_node332.set_neighbor( o_node333.get_node() );

    o_node36.set_child( o_node361.get_node() );
    o_node361.set_neighbor( o_node362.get_node() );
    o_node362.set_neighbor( o_node363.get_node() );

    o_node33.set_neighbor( o_node34.get_node() );
    o_node34.set_neighbor( o_node35.get_node() );
    o_node35.set_neighbor( o_node36.get_node() );

    append_neighbors( &model, o_node1.get_node(), 0);

    let tree_list_model = TreeListModel::new(model,
                                             false /* passthrough */,
                                             true  /* auto expand */,
                                             my_creator);
    let tree_list_model_2 = tree_list_model.clone();

    let selection_model = SingleSelection::new(Some(tree_list_model_2));
    let factory = SignalListItemFactory::new();
    let list_view = ListView::new(Some(selection_model.clone()), Some(factory.clone()));

    let history = OperationHistory::new(list_view.clone());
    let history = Rc::new(history);
    let history_for_factory = history.clone();

    // configuring factory /////////////////////////////////
    // setup handler ///////////////////////////////////////
    factory.connect_setup(move |_, list_item| {
        let expander= TreeExpander::new();
        let label   = Label::new(None);
        list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .set_child(Some(&expander)); // list_item の child は expander
        expander.set_child(Some(&label));

    });

    // bind handler ////////////////////////////////////////
    factory.connect_bind(move |_, list_item| {
        // bindの引数は
        // 1. GtkSignalListItemFactory* self,
        // 2. GObject* object,
        // 3. gpointer user_data

        let expander = list_item.downcast_ref::<ListItem>().expect("Needs to be ListItem")
            .child().and_downcast::<TreeExpander>().expect("The child has to be a `TreeExpander`.");
        let label= expander.child().and_downcast::<Label>().expect("label is expected");

        let tree_list_row = list_item.downcast_ref::<ListItem>().expect("Needs to be ListItem")
            .item().and_downcast::<TreeListRow>().expect("TreeListRow is expected");

        expander.set_list_row( Some(&tree_list_row) );

        // configure label content ///////////////////////
        let scn_object = tree_list_row
            .item()
            .and_downcast::<ScenarioNodeObject>()
            .expect("ScenarioNodeObject is expected");
        label.set_label( &( "id:".to_owned() +
                             &scn_object.get_id().to_string() +
                             ", seq:" +
                             &scn_object.get_seq().to_string() ) );
        label.set_xalign(0.0);
        label.set_vexpand(true); label.set_hexpand(true);

        // configure drag source of label //////////////////

        let drag_source= DragSource::new();
        let scenario_item_drag_source = ScenarioItemDragObject::new();
        scenario_item_drag_source.set_root_store( tree_list_model.model()
                                                          .downcast::<gio::ListStore>()
                                                          .expect("ListStore is expd")) ;
        scenario_item_drag_source.set_history( history_for_factory.clone() );
        scenario_item_drag_source.set_list_item(
            list_item.downcast_ref::<ListItem>().expect("ListItem is expd").clone() );
        drag_source.set_content(
            Some( &ContentProvider::for_value( &Value::from( &scenario_item_drag_source ))));

        drag_source.connect_drag_begin(|_a, _c| { // <DragSource>, <Drag>
            //let widget_paintable= WidgetPaintable::new(glib::bitflags::_core::option::Option::Some(a));
            //a.set_icon(Some(&widget_paintable), 32, 58);
        });
        label.add_controller(drag_source);

        // configure drop target of label //////////////////
        let drop_target= DropTarget::new( ScenarioItemDragObject::static_type(), DragAction::COPY);
        drop_target.connect_drop( label_drop_function );
        drop_target.connect_motion( |d, _x, y|{
            if y < (d.widget().height()/2).into() {
                label_drop_remove_style(d.widget(), true, false);  }
            else {
                label_drop_remove_style(d.widget(), false, true);  }
            DragAction::COPY
        } );
        drop_target.connect_leave(
            |d|{ label_drop_remove_style(d.widget(), false, false); } );

        label.add_controller(drop_target);

        // Expander(リスト行)に対するドロップ(notラベル部分)
        let drop_target2= DropTarget::new( ScenarioItemDragObject::static_type(), DragAction::COPY);
        drop_target2.connect_motion( |d, x, y|{
            let c=
                d.widget()
                .downcast::<TreeExpander>()
                .expect("expander is expected")
                .child().unwrap().allocation();
            let x32 = x as i32;
            let y32 = y as i32;

            if (c.x() <= x32) && (x32 <= c.x() + c.width()) &&
                (c.y() <= y32) && (y32 <= c.y() + c.height()) {
                    label_drop_remove_style(d.widget(), false, false); }
            else if y < (d.widget().height()/2).into() {
                label_drop_remove_style(d.widget(), true, false);  }
            else {
                label_drop_remove_style(d.widget(), false, true);  }
            DragAction::COPY
        } );
        drop_target2.connect_leave(
            |d|{ label_drop_remove_style(d.widget(), false, false); } );
        drop_target2.connect_drop(expander_drop_function);
        expander.add_controller(drop_target2);
    });

    let drop_target= DropTarget::new( ListItem::static_type(), DragAction::COPY);
    drop_target.connect_drop(|d, v, x, y|{
        println!("dropped! d:{:?}, dv:{:?}, v:{:?}, x:{:?}, y:{:?}",

                 //d.widget().downcast::<ListView>().expect("lview is expected").model().unwrap(),
                 d.widget().downcast::<ListView>().expect("lview is expected").first_child(),

                 d.value(), v, x, y);
        true
    });
    list_view.add_controller(drop_target);

    //list_view.set_enable_rubberband(true);
    list_view.set_show_separators(true);

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never) // Disable horizontal scrolling
        .min_content_width(320)
        .min_content_height(480)
        .child(&list_view)
        .build();

    // remove //////////////////////////////////////////////
    let remove_button = Isv2Button::with_label_selection_history("rm",
                                                                 selection_model.clone(),
                                                                 history.clone());
    remove_button.connect_clicked(move |a| {
        if let Ok(hdl) = isv2button_to_dest_member4(a){
            let h= OperationHistoryItem::new_from_handle(Operation::Remove, hdl);
            a.get_history().push(h.clone());
            remove_node(h.src.store.unwrap().as_ref(), h.src.sno.unwrap().as_ref());
        } else {
            println!("empty!");
        }
    });
    // add_neighbor ////////////////////////////////////////
    let add_neighbor_button = Isv2Button::with_label_selection_history("add(n)",
                                                                       selection_model.clone(),
                                                                       history.clone());
    add_neighbor_button.connect_clicked(move |a| {
        let new_node = ScenarioNodeObject::new_with_seq_id(0, get_seq());

        if let Ok(hdl) = isv2button_to_dest_member4(a){
            add_neighbor( hdl.sno.as_ref().unwrap().as_ref(), &new_node, hdl.store.as_ref().unwrap().as_ref() );
            let mut h= OperationHistoryItem::new_from_handle(Operation::AddNeighbor, hdl);
            h.new_sno= Some( Rc::new(new_node.clone()) );
            a.get_history().push(h);
        } else {
            let root_store= a.get_store();
            let h= OperationHistoryItem::new_with_root_store(Operation::AddRoot, &root_store, &new_node);
            a.get_history().push(h);
            add_node_to_empty_store(a.clone(), &new_node);
        }
    });
    // add_child ///////////////////////////////////////////
    let add_child_button = Isv2Button::with_label_selection_history("add(c)",
                                                                    selection_model.clone(),
                                                                    history.clone());
    add_child_button.connect_clicked(move |a| {
        let new_node = ScenarioNodeObject::new_with_seq_id(0, get_seq());
        if let Ok(hdl) = isv2button_to_dest_member4(a){
            add_child( hdl.sno.as_ref().unwrap().as_ref(),
                       &new_node,
                       hdl.row.as_ref().unwrap().as_ref(),
                       hdl.store.as_ref().unwrap().as_ref() );
            let mut h= OperationHistoryItem::new_from_handle(Operation::AddChild, hdl);
            h.new_sno= Some( Rc::new(new_node.clone()) );
            a.get_history().push(h);
        } else {
            let root_store= a.get_store();
            let h= OperationHistoryItem::new_with_root_store(Operation::AddRoot, &root_store, &new_node);
            a.get_history().push(h);
            add_node_to_empty_store(a.clone(), &new_node);
        }
    });
    // undo ////////////////////////////////////////////////
    let undo_button = Isv2Button::with_label_selection_history("undo",
                                                               selection_model.clone(),
                                                               history.clone());
    undo_button.connect_clicked(move |a| {
        a.get_history().undo();
    });
    // redo ////////////////////////////////////////////////
    let redo_button = Isv2Button::with_label_selection_history("redo",
                                                               selection_model.clone(),
                                                               history.clone());
    redo_button.connect_clicked(move |a| {
        a.get_history().redo();
    });
    // update //////////////////////////////////////////////
    let update_button = Button::with_label("update");
    update_button.connect_clicked( |button| {
        let list_view= button.parent().unwrap()
            .prev_sibling().unwrap()
            .first_child().unwrap()
            .downcast::<ListView>().expect("ListView");
        let list_model= list_view.model().unwrap() // SelectionModel
            .downcast::<SingleSelection>().expect("SingleSelection")
            .model().unwrap() // TreeListModel
            .downcast::<TreeListModel>().expect("TreeListModel")
            .model();          // ListModel
        for i in 0..list_model.n_items() {
            list_model.items_changed(i, 1, 1);
        }

    });
    // dump ////////////////////////////////////////////////
    let dump_button = Button::with_label("dump"); // just for debug
    dump_button.connect_clicked( move |_| {
        let obj= list_view.model().unwrap() // SelectionModel
            .downcast::<SingleSelection>().expect("SingleSelection")
            .model().unwrap() // TreeListModel
            .downcast::<TreeListModel>().expect("TreeListModel")
            .model()          // ListModel
            .item(0);         // Object<ScenarioNodeObject>
        let sno= obj.unwrap().downcast_ref::<ScenarioNodeObject>().expect("sno").get_node();
        println!("--------------------");
        sno.dump(0);
        list_view.set_model( Some( &list_view.model().unwrap() ) );
        list_view.queue_draw();

        // 再描画のサンプル，性能はわからないがとりあえず期待通り動作する
        // TODO list_model を move せずに， connect_clicked の引数から生成する

        let list_model= list_view.model().unwrap() // SelectionModel
            .downcast::<SingleSelection>().expect("SingleSelection")
            .model().unwrap() // TreeListModel
            .downcast::<TreeListModel>().expect("TreeListModel")
            .model();          // ListModel
        for i in 0..list_model.n_items() {
            list_model.items_changed(i, 1, 1);
        }

    });

    ////////////////////////////////////////////////////////

    let gtk_box = Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    gtk_box.append(&scrolled_window);

    let button_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .build();
    button_box.append(&undo_button);
    button_box.append(&redo_button);
    button_box.append(&dump_button);
    button_box.append(&update_button);
    button_box.append(&add_neighbor_button);
    button_box.append(&add_child_button);
    button_box.append(&remove_button);
    gtk_box.append(&button_box);

    // Create a window
    let window = ApplicationWindow::builder()
        .application(app)
        .title( String::from("isv2") )
        .default_width(320)
        .default_height(480)
        .child(&gtk_box)
        .build();

    // Present window
    window.present();
}
