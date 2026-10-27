//! The plainest value of whatever a type asks for: what a field is given in the pass that asks
//! whether its struct builds without another, [`super::required`]'s.

use serde::de::{DeserializeSeed, Deserializer, EnumAccess, IntoDeserializer, VariantAccess, Visitor};

use super::{Entries, Given, Halt, Items};

/// The plainest value of whatever is asked for, so a field given to a struct reads as present.
pub(super) struct Anything;

impl<'de> Deserializer<'de> for Anything {
    type Error = Halt;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_str("x")
    }
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_bool(false)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_u64(0)
    }
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_f64(0.0)
    }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_f64(0.0)
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_none()
    }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_unit()
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_seq(Items(None))
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_map(Entries::new())
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Halt> {
        let mut every = Entries::new();
        every.pending.extend(fields.iter().map(|field| (*field, Given::Anything)));
        visitor.visit_map(every)
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Halt> {
        visitor.visit_enum(First(variants.first().copied().unwrap_or_default()))
    }

    // Every integer reads a nought, which each of them can hold.
    serde::forward_to_deserialize_any! {
        char str string bytes byte_buf tuple tuple_struct identifier ignored_any
    }
    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_u64(visitor)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_u64(visitor)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_u64(visitor)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_u64(visitor)
    }
    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_u64(visitor)
    }
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_u64(visitor)
    }
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_u64(visitor)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_u64(visitor)
    }
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_u64(visitor)
    }
}

/// An enum's first variant, carrying the plainest of what it carries.
struct First(&'static str);

impl<'de> EnumAccess<'de> for First {
    type Error = Halt;
    type Variant = Anything;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Anything), Halt> {
        Ok((seed.deserialize(self.0.into_deserializer())?, Anything))
    }
}

impl<'de> VariantAccess<'de> for Anything {
    type Error = Halt;

    fn unit_variant(self) -> Result<(), Halt> {
        Ok(())
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Halt> {
        seed.deserialize(self)
    }
    fn tuple_variant<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_seq(Items(None))
    }
    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_struct("", fields, visitor)
    }
}
