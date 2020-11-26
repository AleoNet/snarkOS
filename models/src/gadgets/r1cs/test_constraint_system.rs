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

use fxhash::{FxBuildHasher, FxHashMap};
use indexmap::IndexSet;

use std::collections::hash_map::Entry;

#[derive(Debug)]
enum NamedObject {
    Constraint(usize),
    Var(Variable),
    Namespace,
}

type ConstraintIdx = usize;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct InternedPath(Vec<usize>);

impl From<Vec<usize>> for InternedPath {
    fn from(v: Vec<usize>) -> Self {
        Self(v)
    }
}

impl AsRef<[usize]> for InternedPath {
    fn as_ref(&self) -> &[usize] {
        &self.0
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct TestConstraint {
    a: ConstraintIdx,
    b: ConstraintIdx,
    c: ConstraintIdx,
}

/// Constraint system for testing purposes.
pub struct TestConstraintSystem<F: Field> {
    interned_path_segments: IndexSet<String, FxBuildHasher>,
    interned_constraints: IndexSet<LinearCombination<F>, FxBuildHasher>,
    named_objects: FxHashMap<InternedPath, NamedObject>,
    current_namespace: InternedPath,
    pub constraints: FxHashMap<InternedPath, TestConstraint>,
    inputs: Vec<(F, InternedPath)>,
    aux: Vec<(F, InternedPath)>,
}

impl<F: Field> TestConstraintSystem<F> {
    fn eval_lc(terms: &[(Variable, F)], inputs: &[(F, InternedPath)], aux: &[(F, InternedPath)]) -> F {
        let mut acc = F::zero();

        for &(var, ref coeff) in terms {
            let mut tmp = match var.get_unchecked() {
                Index::Input(index) => inputs[index].0,
                Index::Aux(index) => aux[index].0,
            };

            tmp.mul_assign(&coeff);
            acc.add_assign(&tmp);
        }

        acc
    }
}

impl<F: Field> Default for TestConstraintSystem<F> {
    fn default() -> Self {
        let mut interned_path_segments = IndexSet::with_hasher(FxBuildHasher::default());
        let path_segment = "ONE".to_owned();
        let path_idx = interned_path_segments.insert_full(path_segment).0;
        let interned_path: InternedPath = vec![path_idx].into();
        let mut named_objects = FxHashMap::default();
        named_objects.insert(
            interned_path.clone(),
            NamedObject::Var(TestConstraintSystem::<F>::one()),
        );

        TestConstraintSystem {
            interned_constraints: IndexSet::with_hasher(FxBuildHasher::default()),
            interned_path_segments,
            named_objects,
            current_namespace: vec![].into(),
            constraints: Default::default(),
            inputs: vec![(F::one(), interned_path)],
            aux: vec![],
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

        for interned_segment in interned_path.as_ref() {
            ret.push_str(self.interned_path_segments.get_index(*interned_segment).unwrap());
        }

        ret
    }

    pub fn print_named_objects(&self) {
        let mut path = String::new();
        for (interned_path, _constraint) in &self.constraints {
            for interned_segment in interned_path.as_ref() {
                path.push_str(self.interned_path_segments.get_index(*interned_segment).unwrap());
            }

            println!("{}", path);
            path.clear();
        }
    }

    pub fn which_is_unsatisfied(&self) -> Option<String> {
        for (interned_path, TestConstraint { a, b, c }) in &self.constraints {
            let a = self.interned_constraints.get_index(*a).unwrap();
            let b = self.interned_constraints.get_index(*b).unwrap();
            let c = self.interned_constraints.get_index(*c).unwrap();

            let mut a = Self::eval_lc(a.as_ref(), &self.inputs, &self.aux);
            let b = Self::eval_lc(b.as_ref(), &self.inputs, &self.aux);
            let c = Self::eval_lc(c.as_ref(), &self.inputs, &self.aux);

            a.mul_assign(&b);

            if a != c {
                return Some(self.unintern_path(interned_path));
            }
        }

        None
    }

    pub fn is_satisfied(&self) -> bool {
        self.which_is_unsatisfied().is_none()
    }

    pub fn num_constraints(&self) -> usize {
        self.constraints.len()
    }

    pub fn set(&mut self, path: &str, to: F) {
        let interned_path = self.intern_path(path);

        match self.named_objects.get(&interned_path) {
            Some(&NamedObject::Var(ref v)) => match v.get_unchecked() {
                Index::Input(index) => self.inputs[index].0 = to,
                Index::Aux(index) => self.aux[index].0 = to,
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

        match self.named_objects.get(&interned_path) {
            Some(&NamedObject::Var(ref v)) => match v.get_unchecked() {
                Index::Input(index) => self.inputs[index].0,
                Index::Aux(index) => self.aux[index].0,
            },
            Some(e) => panic!(
                "tried to get value of path `{}`, but `{:?}` exists there (not a variable)",
                path, e
            ),
            _ => panic!("no variable exists at path: {}", path),
        }
    }

    #[inline]
    fn set_named_obj(&mut self, interned_path: InternedPath, to: NamedObject) {
        match self.named_objects.entry(interned_path) {
            Entry::Vacant(e) => {
                e.insert(to);
            }
            Entry::Occupied(e) => {
                let mut path = String::new();

                for interned_segment in e.remove_entry().0.0 {
                    path.push_str(self.interned_path_segments.get_index(interned_segment).unwrap());
                }

                panic!("tried to create object at existing path: {}", path);
            }
        }
    }

    #[inline]
    fn compute_path(&mut self, new_segment: &str) -> InternedPath {
        assert!(!new_segment.contains('/'), "'/' is not allowed in names");

        let mut vec = Vec::with_capacity(self.current_namespace.as_ref().len() + 1);
        vec.extend_from_slice(self.current_namespace.as_ref());
        let interned_segment = self.interned_path_segments.insert_full(new_segment.to_owned()).0;
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
        let index = self.aux.len();
        let interned_path = self.compute_path(annotation().as_ref());
        self.aux.push((f()?, interned_path.clone()));
        let var = Variable::new_unchecked(Index::Aux(index));
        self.set_named_obj(interned_path, NamedObject::Var(var));

        Ok(var)
    }

    fn alloc_input<Fn, A, AR>(&mut self, annotation: A, f: Fn) -> Result<Variable, SynthesisError>
    where
        Fn: FnOnce() -> Result<F, SynthesisError>,
        A: FnOnce() -> AR,
        AR: AsRef<str>,
    {
        let index = self.inputs.len();
        let interned_path = self.compute_path(annotation().as_ref());
        self.inputs.push((f()?, interned_path.clone()));
        let var = Variable::new_unchecked(Index::Input(index));
        self.set_named_obj(interned_path, NamedObject::Var(var));

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
        let index = self.constraints.len();
        self.set_named_obj(interned_path.clone(), NamedObject::Constraint(index));

        let mut a = a(LinearCombination::zero());
        let mut b = b(LinearCombination::zero());
        let mut c = c(LinearCombination::zero());
        a.0.shrink_to_fit();
        b.0.shrink_to_fit();
        c.0.shrink_to_fit();

        self.interned_constraints.reserve(3);
        let a = self.interned_constraints.insert_full(a).0;
        let b = self.interned_constraints.insert_full(b).0;
        let c = self.interned_constraints.insert_full(c).0;

        self.constraints.insert(interned_path, TestConstraint { a, b, c });
    }

    fn push_namespace<NR: AsRef<str>, N: FnOnce() -> NR>(&mut self, name_fn: N) {
        let name = name_fn();
        let interned_path = self.compute_path(name.as_ref());
        let new_segment = interned_path.as_ref().last().unwrap().clone();
        self.set_named_obj(interned_path, NamedObject::Namespace);
        self.current_namespace.0.push(new_segment);
    }

    fn pop_namespace(&mut self) {
        assert!(self.current_namespace.0.pop().is_some());
    }

    fn get_root(&mut self) -> &mut Self::Root {
        self
    }

    fn num_constraints(&self) -> usize {
        self.constraints.len()
    }
}
