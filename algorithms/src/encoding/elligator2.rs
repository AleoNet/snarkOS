use snarkos_errors::algorithms::EncodingError;
use snarkos_models::curves::{
    pairing_engine::{AffineCurve, ProjectiveCurve},
    Field,
    Group,
    LegendreSymbol,
    MontgomeryModelParameters,
    One,
    SquareRootField,
    TEModelParameters,
    Zero,
};
use snarkos_utilities::{to_bytes, FromBytes, ToBytes};

pub struct Elligator2 {}

impl Elligator2 {
    pub fn encode<P: MontgomeryModelParameters + TEModelParameters, G: Group + ProjectiveCurve>(
        fr_element: <G as Group>::ScalarField,
    ) -> Result<<G as ProjectiveCurve>::Affine, EncodingError> {
        println!("Starting Fr element is - {:?}", fr_element);

        // Map it to its corresponding Fq element.
        let fq_element = {
            let output = P::BaseField::from_random_bytes(&to_bytes![fr_element]?);
            assert!(output.is_some());
            output.unwrap()
        };

        let A = <P as MontgomeryModelParameters>::COEFF_A;
        let B = <P as MontgomeryModelParameters>::COEFF_B;

        // Compute the parameters for the alternate Montgomery form: v^2 == u^3 + A * u^2 + B * u.
        let (a, b) = {
            let a = A * &B.inverse().unwrap();
            let b = P::BaseField::one() * &B.square().inverse().unwrap();
            (a, b)
        };

        println!("Starting Fq element is {:?}", fq_element);

        // Compute the mapping from Fq to E(Fq) as an alternate Montgomery element (u, v).
        let (u, v) = {
            // Let r = element.
            let r = fq_element;

            // Let u = D.
            // TODO (howardwu): change to 5.
            let u = <P as TEModelParameters>::COEFF_D;

            // Let ur2 = u * r^2;
            let ur2 = r.square() * &u;

            {
                // Verify r is nonzero.
                assert!(!r.is_zero());

                // Verify u is a quadratic nonresidue.
                assert!(u.legendre().is_qnr());

                // Verify 1 + ur^2 != 0.
                assert_ne!(P::BaseField::one() + &ur2, P::BaseField::zero());

                // Verify A^2 * ur^2 != B(1 + ur^2)^2.
                let a2 = a.square();
                assert_ne!(a2 * &ur2, (P::BaseField::one() + &ur2).square() * &b);
            }

            // Let v = -A / (1 + ur^2).
            let v = (P::BaseField::one() + &ur2).inverse().unwrap() * &(-a);

            // Let e = legendre(v^3 + Av^2 + Bv).
            let v2 = v.square();
            let v3 = v2 * &v;
            let av2 = a.clone() * &v2;
            let bv = b.clone() * &v;
            let e = (v3 + &(av2 + &bv)).legendre();

            // Let x = ev - ((1 - e) * A/2).
            let two = P::BaseField::one().double();
            let x = match e {
                LegendreSymbol::Zero => -(a.clone() * &two.inverse().unwrap()),
                LegendreSymbol::QuadraticResidue => v,
                LegendreSymbol::QuadraticNonResidue => (-v) - &a,
            };

            // Let y = -e * sqrt(x^3 + Ax^2 + Bx).
            let x2 = x.square();
            let x3 = x2 * &x;
            let ax2 = a.clone() * &x2;
            let bx = b.clone() * &x;
            let value = (x3 + &(ax2 + &bx)).sqrt().unwrap();
            let y = match e {
                LegendreSymbol::Zero => P::BaseField::zero(),
                LegendreSymbol::QuadraticResidue => -value,
                LegendreSymbol::QuadraticNonResidue => value,
            };

            (x, y)
        };

        // Ensure (u, v) is a valid alternate Montgomery element.
        {
            // Enforce v^2 == u^3 + A * u^2 + B * u
            let v2 = v.square();
            let u2 = u.square();
            let u3 = u2 * &u;
            assert_eq!(v2, u3 + &(a * &u2) + &(b * &u));
        }

        // Convert the alternate Montgomery element (u, v) to Montgomery element (s, t).
        let (s, t) = {
            let s = u * &B;
            let t = v * &B;

            // Ensure (s, t) is a valid Montgomery element
            {
                // Enforce B * t^2 == s^3 + A * s^2 + s
                let t2 = t.square();
                let s2 = s.square();
                let s3 = s2 * &s;
                assert_eq!(B * &t2, s3 + &(A * &s2) + &s);
            }

            (s, t)
        };

        // Convert the Montgomery element (s, t) to the twisted Edwards element (x, y).
        let (x, y) = {
            let x = s * &t.inverse().unwrap();

            let numerator = s - &P::BaseField::one();
            let denominator = s + &P::BaseField::one();
            let y = numerator * &denominator.inverse().unwrap();

            (x, y)
        };

        let group_recovered = <G as ProjectiveCurve>::Affine::from_random_bytes(&to_bytes![x].unwrap()).unwrap();

        assert_eq!(to_bytes![x]?, to_bytes![group_recovered.to_x_coordinate()]?);
        assert_eq!(to_bytes![y]?, to_bytes![group_recovered.to_y_coordinate()]?);

        Ok(group_recovered)
    }

    pub fn decode<P: MontgomeryModelParameters + TEModelParameters, G: Group + ProjectiveCurve>(
        group_element: <G as ProjectiveCurve>::Affine,
    ) -> Result<(), EncodingError> {
        unimplemented!()
    }
}
