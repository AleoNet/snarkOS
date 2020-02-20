use crate::templates::{
    short_weierstrass::short_weierstrass_jacobian::{GroupAffine as SWAffine, GroupProjective as SWProjective},
    twisted_edwards_extended::{GroupAffine as TEAffine, GroupProjective as TEProjective},
};
use snarkos_models::curves::{
    to_field_vec::{Error, ToConstraintField},
    Field,
    ProjectiveCurve,
    SWModelParameters,
    TEModelParameters,
};

impl<M: TEModelParameters, F: Field> ToConstraintField<F> for TEAffine<M>
where
    M::BaseField: ToConstraintField<F>,
{
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, Error> {
        let mut x_fe = self.x.to_field_elements()?;
        let y_fe = self.y.to_field_elements()?;
        x_fe.extend_from_slice(&y_fe);
        Ok(x_fe)
    }
}

impl<M: TEModelParameters, F: Field> ToConstraintField<F> for TEProjective<M>
where
    M::BaseField: ToConstraintField<F>,
{
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, Error> {
        let affine = self.into_affine();
        let mut x_fe = affine.x.to_field_elements()?;
        let y_fe = affine.y.to_field_elements()?;
        x_fe.extend_from_slice(&y_fe);
        Ok(x_fe)
    }
}

impl<M: SWModelParameters, F: Field> ToConstraintField<F> for SWAffine<M>
where
    M::BaseField: ToConstraintField<F>,
{
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, Error> {
        let mut x_fe = self.x.to_field_elements()?;
        let y_fe = self.y.to_field_elements()?;
        x_fe.extend_from_slice(&y_fe);
        Ok(x_fe)
    }
}

impl<M: SWModelParameters, F: Field> ToConstraintField<F> for SWProjective<M>
where
    M::BaseField: ToConstraintField<F>,
{
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, Error> {
        let affine = self.into_affine();
        let mut x_fe = affine.x.to_field_elements()?;
        let y_fe = affine.y.to_field_elements()?;
        x_fe.extend_from_slice(&y_fe);
        Ok(x_fe)
    }
}
