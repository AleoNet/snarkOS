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
use indexmap::IndexSet;
use nohash_hasher::IntMap;

use std::collections::hash_map::Entry;

#[derive(Debug)]
enum NamedObject {
    Constraint(usize),
    Var(Variable),
    Namespace,
}

type PathIdx = usize;
type TestConstraint<T> = (
    LinearCombination<T>,
    LinearCombination<T>,
    LinearCombination<T>,
    PathIdx,
);

/// Constraint system for testing purposes.
pub struct TestConstraintSystem<F: Field> {
    paths: IndexSet<String, FxBuildHasher>,
    named_objects: IntMap<PathIdx, NamedObject>,
    current_namespace: Vec<String>,
    pub constraints: Vec<TestConstraint<F>>,
    inputs: Vec<(F, PathIdx)>,
    aux: Vec<(F, PathIdx)>,
}

impl<F: Field> TestConstraintSystem<F> {
    fn eval_lc(terms: &[(Variable, F)], inputs: &[(F, PathIdx)], aux: &[(F, PathIdx)]) -> F {
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
        let mut paths = IndexSet::with_hasher(FxBuildHasher::default());
        let path_idx = paths.insert_full("ONE".into()).0;
        let mut map = IntMap::default();
        map.insert(path_idx, NamedObject::Var(TestConstraintSystem::<F>::one()));

        TestConstraintSystem {
            paths,
            named_objects: map,
            current_namespace: vec![],
            constraints: vec![],
            inputs: vec![(F::one(), path_idx)],
            aux: vec![],
        }
    }
}

impl<F: Field> TestConstraintSystem<F> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn print_named_objects(&self) {
        for &(_, _, _, path_idx) in &self.constraints {
            println!("{}", self.paths.get_index(path_idx).unwrap());
        }
    }

    pub fn which_is_unsatisfied(&self) -> Option<&str> {
        for &(ref a, ref b, ref c, path_idx) in &self.constraints {
            let mut a = Self::eval_lc(a.as_ref(), &self.inputs, &self.aux);
            let b = Self::eval_lc(b.as_ref(), &self.inputs, &self.aux);
            let c = Self::eval_lc(c.as_ref(), &self.inputs, &self.aux);

            a.mul_assign(&b);

            if a != c {
                return self.paths.get_index(path_idx).map(|p| p.as_str());
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
        let path_idx = self
            .paths
            .get_index_of(path)
            .unwrap_or_else(|| panic!("no variable exists at path: {}", path));

        match self.named_objects.get(&path_idx) {
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
        let path_idx = self
            .paths
            .get_index_of(path)
            .unwrap_or_else(|| panic!("no variable exists at path: {}", path));

        match self.named_objects.get(&path_idx) {
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
    fn set_named_obj(&mut self, path_idx: PathIdx, to: NamedObject) {
        match self.named_objects.entry(path_idx) {
            Entry::Vacant(e) => {
                e.insert(to);
            }
            Entry::Occupied(e) => {
                panic!("tried to create object at existing path: {}", e.key());
            }
        }
    }
}

#[inline]
fn compute_path(ns: &[String], this: &str) -> String {
    assert!(!this.contains('/'), "'/' is not allowed in names");

    // preallocate the target path size, including the separators
    let len = ns.iter().map(|s| s.len()).sum::<usize>() + ns.len() + this.len();
    let mut name = String::with_capacity(len);

    for (i, ns) in ns.iter().map(|s| s.as_str()).chain(Some(this)).enumerate() {
        if i != 0 {
            name.push('/');
        }
        name.push_str(ns);
    }

    name
}

impl<F: Field> ConstraintSystem<F> for TestConstraintSystem<F> {
    type Root = Self;

    fn alloc<Fn, A, AR>(&mut self, annotation: A, f: Fn) -> Result<Variable, SynthesisError>
    where
        Fn: FnOnce() -> Result<F, SynthesisError>,
        A: FnOnce() -> AR,
        AR: Into<String>,
    {
        let index = self.aux.len();
        let path = compute_path(&self.current_namespace, &annotation().into());
        let path_idx = self.paths.insert_full(path).0;
        self.aux.push((f()?, path_idx));
        let var = Variable::new_unchecked(Index::Aux(index));
        self.set_named_obj(path_idx, NamedObject::Var(var));

        Ok(var)
    }

    fn alloc_input<Fn, A, AR>(&mut self, annotation: A, f: Fn) -> Result<Variable, SynthesisError>
    where
        Fn: FnOnce() -> Result<F, SynthesisError>,
        A: FnOnce() -> AR,
        AR: Into<String>,
    {
        let index = self.inputs.len();
        let path = compute_path(&self.current_namespace, &annotation().into());
        let path_idx = self.paths.insert_full(path).0;
        self.inputs.push((f()?, path_idx));
        let var = Variable::new_unchecked(Index::Input(index));
        self.set_named_obj(path_idx, NamedObject::Var(var));

        Ok(var)
    }

    fn enforce<A, AR, LA, LB, LC>(&mut self, annotation: A, a: LA, b: LB, c: LC)
    where
        A: FnOnce() -> AR,
        AR: Into<String>,
        LA: FnOnce(LinearCombination<F>) -> LinearCombination<F>,
        LB: FnOnce(LinearCombination<F>) -> LinearCombination<F>,
        LC: FnOnce(LinearCombination<F>) -> LinearCombination<F>,
    {
        let path = compute_path(&self.current_namespace, &annotation().into());
        let path_idx = self.paths.insert_full(path).0;
        let index = self.constraints.len();
        self.set_named_obj(path_idx, NamedObject::Constraint(index));

        let mut a = a(LinearCombination::zero());
        let mut b = b(LinearCombination::zero());
        let mut c = c(LinearCombination::zero());
        a.0.shrink_to_fit();
        b.0.shrink_to_fit();
        c.0.shrink_to_fit();

        self.constraints.push((a, b, c, path_idx));
    }

    fn push_namespace<NR: Into<String>, N: FnOnce() -> NR>(&mut self, name_fn: N) {
        let name = name_fn().into();
        let path = compute_path(&self.current_namespace, &name);
        let path_idx = self.paths.insert_full(path).0;
        self.set_named_obj(path_idx, NamedObject::Namespace);
        self.current_namespace.push(name);
    }

    fn pop_namespace(&mut self) {
        assert!(self.current_namespace.pop().is_some());
    }

    fn get_root(&mut self) -> &mut Self::Root {
        self
    }

    fn num_constraints(&self) -> usize {
        self.constraints.len()
    }
}
