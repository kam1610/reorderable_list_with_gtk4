mod imp;

use glib::Object;
use gtk::glib;
use gtk::SingleSelection;
use gtk::{gio, TreeListModel};
use gtk::prelude::Cast;
use glib::subclass::types::ObjectSubclassIsExt;
use std::rc::Rc;
use crate::operation_history::OperationHistory;

glib::wrapper! {
    pub struct Isv2Button(ObjectSubclass<imp::Isv2Button>)
        @extends gtk::Button, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl Isv2Button {
    pub fn new() -> Self {
        Object::builder().build()
    }
    pub fn with_label(label: &str) -> Self {
        let obj: Isv2Button= Object::builder().property("label", label).build();
        obj
    }
    pub fn with_label_selection(label: &str, selection: SingleSelection) -> Self {
        let obj: Isv2Button= Isv2Button::with_label(label);
        obj.set_selection(selection);
        obj
    }
    pub fn with_label_selection_history(label    : &str,
                                        selection: SingleSelection,
                                        history  : Rc<OperationHistory>
    ) -> Self {
        let obj: Isv2Button= Isv2Button::with_label(label);
        obj.set_selection(selection);
        obj.set_history(history);
        obj
    }
    pub fn set_selection(&self, s: SingleSelection){
        *self.imp().selection.borrow_mut()= s.into();
    }
    pub fn get_selection(&self) -> Rc<SingleSelection>{
        self.imp().selection.borrow().clone()
    }
    pub fn set_history(&self, h: Rc<OperationHistory>){
        *self.imp().history.borrow_mut()= h.into();
    }
    pub fn get_history(&self) -> Rc<OperationHistory>{
        self.imp().history.borrow().clone()
    }
    pub fn get_store(&self) -> gio::ListStore {
        self.get_selection().model().unwrap()
            .downcast::<TreeListModel>().expect("TreeListModel").model()
            .downcast::<gio::ListStore>().expect("ListStore")
    }

}

impl Default for Isv2Button {
    fn default() -> Self {
        Self::new()
    }
}
