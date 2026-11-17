export function guiDescription(value = 0, kind = 'integer') {
  const typed = number => kind === 'integer'
    ? { variant_tag: 1, data: { kind: 'integer', value: number } }
    : { variant_tag: 2, data: { kind: 'float', value: number, bits_hex: number ? '3FF0000000000000' : '0000000000000000' } };
  return { name: 'autofill', title: 'autofill', tag: 4, value, text: String(value), min: 0, max: 1, default: 0,
    normalized: value, enabled: true, readOnly: false, writable: true, native_kind: 'Toggle', native_value: typed(value),
    native_min: typed(0), native_max: typed(1), native_default: typed(0),
    native_normalized: { kind: 'float32', bits_hex: value ? '3F800000' : '00000000' },
    native_support: { raw_number: true, normalized: true, display_text: true },
    support: { raw: true, normalized: true, text: true }, display_parser: 'PlainNumeric' };
}
