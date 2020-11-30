// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    curves::Field,
    gadgets::r1cs::{ConstraintSystem, Index, LinearCombination, Variable},
};
use snarkos_errors::gadgets::SynthesisError;

use fxhash::FxBuildHasher;
use indexmap::{map::Entry, IndexMap, IndexSet};

use std::{borrow::Borrow, collections::VecDeque, ops::Deref, rc::Rc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NamedObject {
    Constraint(usize),
    Var(Variable),
    Namespace(Vec<NamedObject>),
}

type Holes = VecDeque<usize>;
type InternedConstraint = usize;
type InternedField = usize;
type InternedLC = Vec<(Variable, InternedField)>;
type InternedPathSegment = usize;
type NamespaceIndex = usize;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct InternedPath(Rc<Vec<usize>>);

impl From<Vec<usize>> for InternedPath {
    fn from(v: Vec<usize>) -> Self {
        Self(Rc::new(v))
    }
}

impl Deref for InternedPath {
    type Target = [usize];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<Vec<usize>> for InternedPath {
    fn borrow(&self) -> &Vec<usize> {
        &self.0
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct TestConstraint {
    a: InternedConstraint,
    b: InternedConstraint,
    c: InternedConstraint,
}

/// Constraint system for testing purposes.
pub struct TestConstraintSystem<F: Field> {
    interned_path_segments: IndexSet<String, FxBuildHasher>,
    interned_fields: IndexSet<F, FxBuildHasher>,
    interned_constraints: IndexSet<InternedLC, FxBuildHasher>,
    named_objects: IndexMap<InternedPath, NamedObject, FxBuildHasher>,
    current_namespace: (Vec<InternedPathSegment>, NamespaceIndex),
    constraints: (Vec<Option<(InternedPath, TestConstraint)>>, Holes),
    inputs: (Vec<Option<InternedField>>, Holes),
    aux: (Vec<Option<InternedField>>, Holes),
}

impl<F: Field> Default for TestConstraintSystem<F> {
    fn default() -> Self {
        let mut interned_path_segments = IndexSet::with_hasher(FxBuildHasher::default());
        let path_segment = "ONE".to_owned();
        let interned_path_segment = interned_path_segments.insert_full(path_segment).0;
        let interned_path: InternedPath = vec![interned_path_segment].into();
        let mut named_objects = IndexMap::with_hasher(FxBuildHasher::default());
        named_objects
            .insert_full(interned_path, NamedObject::Var(TestConstraintSystem::<F>::one()))
            .0;
        let mut interned_fields = IndexSet::with_hasher(FxBuildHasher::default());
        let interned_field = interned_fields.insert_full(F::one()).0;

        TestConstraintSystem {
            interned_fields,
            interned_constraints: IndexSet::with_hasher(FxBuildHasher::default()),
            interned_path_segments,
            named_objects,
            current_namespace: (vec![], 0),
            constraints: Default::default(),
            inputs: (vec![Some(interned_field)], Default::default()),
            aux: Default::default(),
        }
    }
}

impl<F: Field> TestConstraintSystem<F> {
    pub fn new() -> Self {
        Self::default()
    }

    fn intern_path(&self, path: &str) -> InternedPath {
        let mut vec = vec![];

        for segment in path.split('/') {
            vec.push(self.interned_path_segments.get_index_of(segment).unwrap());
        }

        vec.into()
    }

    fn unintern_path(&self, interned_path: &InternedPath) -> String {
        let mut ret = String::new();
        let mut iter = interned_path.iter().peekable();

        while let Some(interned_segment) = iter.next() {
            ret.push_str(self.interned_path_segments.get_index(*interned_segment).unwrap());
            if iter.peek().is_some() {
                ret.push('/');
            }
        }

        ret
    }

    pub fn print_named_objects(&self) {
        let mut path = String::new();
        for (interned_path, _) in self
            .constraints
            .0
            .iter()
            .filter(|c| c.is_some())
            .map(|c| c.as_ref().unwrap())
        {
            for interned_segment in interned_path.iter() {
                path.push_str(self.interned_path_segments.get_index(*interned_segment).unwrap());
            }

            println!("{}", path);
            path.clear();
        }
    }

    fn eval_lc(&self, terms: &[(Variable, InternedField)]) -> F {
        let mut acc = F::zero();

        for &(var, interned_coeff) in terms {
            let interned_tmp = match var.get_unchecked() {
                Index::Input(index) => self.inputs.0[index].unwrap(),
                Index::Aux(index) => self.aux.0[index].unwrap(),
            };
            let mut tmp = *self.interned_fields.get_index(interned_tmp).unwrap();
            let coeff = self.interned_fields.get_index(interned_coeff).unwrap();

            tmp.mul_assign(coeff);
            acc.add_assign(&tmp);
        }

        acc
    }

    pub fn which_is_unsatisfied(&self) -> Option<String> {
        for (interned_path, TestConstraint { a, b, c }) in self
            .constraints
            .0
            .iter()
            .filter(|c| c.is_some())
            .map(|c| c.as_ref().unwrap())
        {
            let a = self.interned_constraints.get_index(*a).unwrap();
            let b = self.interned_constraints.get_index(*b).unwrap();
            let c = self.interned_constraints.get_index(*c).unwrap();

            let mut a = self.eval_lc(a.as_ref());
            let b = self.eval_lc(b.as_ref());
            let c = self.eval_lc(c.as_ref());

            a.mul_assign(&b);

            if a != c {
                return Some(self.unintern_path(&interned_path));
            }
        }

        None
    }

    pub fn is_satisfied(&self) -> bool {
        self.which_is_unsatisfied().is_none()
    }

    pub fn num_constraints(&self) -> usize {
        self.constraints.0.iter().filter(|c| c.is_some()).count()
    }

    pub fn set(&mut self, path: &str, to: F) {
        let interned_path = self.intern_path(path);
        let interned_field = self.interned_fields.insert_full(to).0;

        match self.named_objects.get(&interned_path) {
            Some(&NamedObject::Var(ref v)) => match v.get_unchecked() {
                Index::Input(index) => self.inputs.0[index] = Some(interned_field),
                Index::Aux(index) => self.aux.0[index] = Some(interned_field),
            },
            Some(e) => panic!(
                "tried to set path `{}` to value, but `{:?}` already exists there.",
                path, e
            ),
            _ => panic!("no variable exists at path: {}", path),
        }
    }

    pub fn get(&mut self, path: &str) -> F {
        let interned_path = self.intern_path(path);

        let interned_field = match self.named_objects.get(&interned_path) {
            Some(&NamedObject::Var(ref v)) => match v.get_unchecked() {
                Index::Input(index) => self.inputs.0[index].unwrap(),
                Index::Aux(index) => self.aux.0[index].unwrap(),
            },
            Some(e) => panic!(
                "tried to get value of path `{}`, but `{:?}` exists there (not a variable)",
                path, e
            ),
            _ => panic!("no variable exists at path: {}", path),
        };

        *self.interned_fields.get_index(interned_field).unwrap()
    }

    #[inline]
    fn set_named_obj(&mut self, interned_path: InternedPath, to: NamedObject) -> NamespaceIndex {
        match self.named_objects.entry(interned_path) {
            Entry::Vacant(e) => {
                let ns_idx = e.index();
                //println!("  set_named_obj {:?} : {}", &to, ns_idx);
                e.insert(to);
                ns_idx
            }
            Entry::Occupied(e) => {
                let mut path = String::new();
                for interned_segment in (e.remove_entry().0).iter() {
                    path.push_str(self.interned_path_segments.get_index(*interned_segment).unwrap());
                }

                panic!("tried to create object at existing path: {}", path);
            }
        }
    }

    #[inline]
    fn compute_path(&mut self, new_segment: &str) -> InternedPath {
        let mut vec = Vec::with_capacity(self.current_namespace.0.len() + 1);
        vec.extend_from_slice(&self.current_namespace.0);
        let (interned_segment, new) = self.interned_path_segments.insert_full(new_segment.to_owned());

        // only perform the check for segments not seen before
        assert!(!new || !new_segment.contains('/'), "'/' is not allowed in names");

        vec.push(interned_segment);

        vec.into()
    }
}

impl<F: Field> ConstraintSystem<F> for TestConstraintSystem<F> {
    type Root = Self;

    fn alloc<Fn, A, AR>(&mut self, annotation: A, f: Fn) -> Result<Variable, SynthesisError>
    where
        Fn: FnOnce() -> Result<F, SynthesisError>,
        A: FnOnce() -> AR,
        AR: AsRef<str>,
    {
        let index = self.aux.1.pop_front().unwrap_or_else(|| self.aux.0.len());
        let interned_path = self.compute_path(annotation().as_ref());
        let interned_field = self.interned_fields.insert_full(f()?).0;
        if index < self.aux.0.len() {
            self.aux.0[index] = Some(interned_field);
        } else {
            self.aux.0.push(Some(interned_field));
        }
        let var = Variable::new_unchecked(Index::Aux(index));
        let named_obj = NamedObject::Var(var);
        if let NamedObject::Namespace(ref mut ns) =
            self.named_objects.get_index_mut(self.current_namespace.1).unwrap().1
        {
            ns.push(named_obj.clone());
        }
        self.set_named_obj(interned_path, named_obj);

        Ok(var)
    }

    fn alloc_input<Fn, A, AR>(&mut self, annotation: A, f: Fn) -> Result<Variable, SynthesisError>
    where
        Fn: FnOnce() -> Result<F, SynthesisError>,
        A: FnOnce() -> AR,
        AR: AsRef<str>,
    {
        let index = self.inputs.1.pop_front().unwrap_or_else(|| self.inputs.0.len());
        let interned_path = self.compute_path(annotation().as_ref());
        let interned_field = self.interned_fields.insert_full(f()?).0;
        if index < self.inputs.0.len() {
            self.inputs.0[index] = Some(interned_field);
        } else {
            self.inputs.0.push(Some(interned_field));
        }
        let var = Variable::new_unchecked(Index::Input(index));
        let named_obj = NamedObject::Var(var);
        if let NamedObject::Namespace(ref mut ns) =
            self.named_objects.get_index_mut(self.current_namespace.1).unwrap().1
        {
            ns.push(named_obj.clone());
        }
        self.set_named_obj(interned_path, named_obj);

        Ok(var)
    }

    fn enforce<A, AR, LA, LB, LC>(&mut self, annotation: A, a: LA, b: LB, c: LC)
    where
        A: FnOnce() -> AR,
        AR: AsRef<str>,
        LA: FnOnce(LinearCombination<F>) -> LinearCombination<F>,
        LB: FnOnce(LinearCombination<F>) -> LinearCombination<F>,
        LC: FnOnce(LinearCombination<F>) -> LinearCombination<F>,
    {
        let interned_path = self.compute_path(annotation().as_ref());
        let index = self
            .constraints
            .1
            .pop_front()
            .unwrap_or_else(|| self.constraints.0.len());
        let named_obj = NamedObject::Constraint(index);
        if let NamedObject::Namespace(ref mut ns) =
            self.named_objects.get_index_mut(self.current_namespace.1).unwrap().1
        {
            ns.push(named_obj.clone());
        }
        self.set_named_obj(interned_path.clone(), named_obj);

        let a = a(LinearCombination::zero());
        let a =
            a.0.into_iter()
                .map(|(var, field)| {
                    let interned_field = self.interned_fields.insert_full(field).0;
                    (var, interned_field)
                })
                .collect();
        let a = self.interned_constraints.insert_full(a).0;

        let b = b(LinearCombination::zero());
        let b =
            b.0.into_iter()
                .map(|(var, field)| {
                    let interned_field = self.interned_fields.insert_full(field).0;
                    (var, interned_field)
                })
                .collect();
        let b = self.interned_constraints.insert_full(b).0;

        let c = c(LinearCombination::zero());
        let c =
            c.0.into_iter()
                .map(|(var, field)| {
                    let interned_field = self.interned_fields.insert_full(field).0;
                    (var, interned_field)
                })
                .collect();
        let c = self.interned_constraints.insert_full(c).0;

        if index < self.constraints.0.len() {
            self.constraints.0[index] = Some((interned_path, TestConstraint { a, b, c }));
        } else {
            self.constraints
                .0
                .push(Some((interned_path, TestConstraint { a, b, c })));
        }
    }

    fn push_namespace<NR: AsRef<str>, N: FnOnce() -> NR>(&mut self, name_fn: N) {
        let name = name_fn();
        let interned_path = self.compute_path(name.as_ref());
        let new_segment = *interned_path.0.last().unwrap();
        let namespace_idx = self.set_named_obj(interned_path.clone(), NamedObject::Namespace(vec![])); // FIXME: remove this clone() after debugging

        //println!("pushed ns {} : {}", namespace_idx, self.unintern_path(&interned_path));
        self.current_namespace.0.push(new_segment);
        self.current_namespace.1 = namespace_idx;
        //println!("  curr ns idx: {}", namespace_idx);
    }

    fn pop_namespace(&mut self) {
        let (current_ns, ns_idx) = &self.current_namespace;
        //println!("popping ns {} : {}", ns_idx, self.unintern_path(&current_ns.to_owned().into()));

        let named_object = if let NamedObject::Namespace(no) = self.named_objects.swap_remove_index(*ns_idx).unwrap().1
        // FIXME: should be ok to change to swap_remove_index
        {
            no
        } else {
            unreachable!()
        };

        for child_obj in named_object {
            match child_obj {
                NamedObject::Var(var) => match var.get_unchecked() {
                    Index::Aux(idx) => {
                        self.aux.0[idx] = None;
                        self.aux.1.push_back(idx);
                        //println!("  removing Aux({})", idx);
                    }
                    Index::Input(idx) => {
                        self.inputs.0[idx] = None;
                        self.inputs.1.push_back(idx);
                        //println!("  removing Input({})", idx);
                    }
                },
                NamedObject::Constraint(idx) => {
                    self.constraints.0[idx] = None;
                    self.constraints.1.push_back(idx);
                    //println!("  removing Constraint({})", idx);
                }
                _ => {}
            }
        }

        assert!(self.current_namespace.0.pop().is_some());
        if let Some(new_ns_idx) = self.named_objects.get_index_of(&self.current_namespace.0) {
            self.current_namespace.1 = new_ns_idx;
            //println!("  curr ns idx: {}", new_ns_idx);
        } else {
            // we must be at the "bottom" namespace
            self.current_namespace.1 = 0;
            //println!("  curr ns idx: 0");
        }
    }

    fn get_root(&mut self) -> &mut Self::Root {
        self
    }

    fn num_constraints(&self) -> usize {
        self.num_constraints()
    }
}
