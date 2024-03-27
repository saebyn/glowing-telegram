import { expect, describe, it } from "vitest";
import floatJsonSerializer, { Float } from "./floatJson";

describe("Float", () => {
  it("should keep one decimal place if the value is a whole number", () => {
    const value = new Float(1);
    expect(value.toString()).toBe("1.0");
  });

  it("should keep one decimal place if the value has one digit", () => {
    const value = new Float(1.1);
    expect(value.toString()).toBe("1.1");
  });

  it("should round the value to 4 decimal places", () => {
    const value = new Float(1.23456789);
    expect(value.toString()).toBe("1.2346");
  });

  it("should round the value to 4 decimal places with a negative number", () => {
    const value = new Float(-1.23456789);
    expect(value.toString()).toBe("-1.2346");
  });

  it("should round the value to 4 decimal places with a negative whole number", () => {
    const value = new Float(-1);
    expect(value.toString()).toBe("-1.0");
  });

  it("should round the value to 4 decimal places with a number with less than 4 decimal places", () => {
    const value = new Float(1.23);
    expect(value.toString()).toBe("1.23");
  });

  it("should round the value to 4 decimal places with a negative number with less than 4 decimal places", () => {
    const value = new Float(-1.23);
    expect(value.toString()).toBe("-1.23");
  });
});

describe("floatJsonSerializer", () => {
  it("should serialize a Float object", () => {
    const value = { a: new Float(1.234) };
    expect(floatJsonSerializer(value)).toBe('{"a": 1.234}');
  });

  it("should serialize an array of Float objects", () => {
    const value = { a: [new Float(1.234), new Float(5.678)] };
    expect(floatJsonSerializer(value)).toBe('{"a": [1.234, 5.678]}');
  });

  it("should serialize an object with Float properties", () => {
    const value = { a: new Float(1.234), b: new Float(5.678) };
    expect(floatJsonSerializer(value)).toBe('{"a": 1.234, "b": 5.678}');
  });

  it("should serialize a nested object with Float properties", () => {
    const value = { a: new Float(1.234), b: { c: new Float(5.678) } };
    expect(floatJsonSerializer(value)).toBe('{"a": 1.234, "b": {"c": 5.678}}');
  });

  it("should serialize a nested array of Float objects", () => {
    const value = { a: [new Float(1.234), [new Float(5.678)]] };
    expect(floatJsonSerializer(value)).toBe('{"a": [1.234, [5.678]]}');
  });

  it("should serialize a nested object with Float properties and array of Float objects", () => {
    const value = {
      a: new Float(1.234),
      b: { c: new Float(5.678), d: [new Float(9.012)] },
    };
    expect(floatJsonSerializer(value)).toBe(
      '{"a": 1.234, "b": {"c": 5.678, "d": [9.012]}}'
    );
  });

  it("should support the space parameter of JSON.stringify", () => {
    const value = { a: new Float(1.234), b: new Float(5.678) };
    expect(floatJsonSerializer(value, null, 2)).toBe(
      `{
  "a": 1.234,
  "b": 5.678
}`
    );
  });

  it("should serialize an array with a space parameter", () => {
    const value = [new Float(1.234), new Float(5.678)];
    expect(floatJsonSerializer(value, null, 2)).toBe(
      `[
  1.234,
  5.678
]`
    );
  });

  it("should serialize a nested object with a space parameter", () => {
    const value = { a: new Float(1.234), b: { c: new Float(5.678) } };
    expect(floatJsonSerializer(value, null, 2)).toBe(
      `{
  "a": 1.234,
  "b": {
    "c": 5.678
  }
}`
    );
  });

  it("should serialize a nested array of Float objects with a space parameter", () => {
    const value = { a: [new Float(1.234), [new Float(5.678)]] };
    expect(floatJsonSerializer(value, null, 2)).toBe(
      `{
  "a": [
    1.234,
    [
      5.678
    ]
  ]
}`
    );
  });

  it("should serialize an empty object", () => {
    const value = {};
    expect(floatJsonSerializer(value)).toBe("{}");
  });

  it("should serialize an empty array", () => {
    const value: never[] = [];
    expect(floatJsonSerializer(value)).toBe("[]");
  });

  it("should serialize an empty object with a space parameter", () => {
    const value = {};
    expect(floatJsonSerializer(value, null, 2)).toBe("{}");
  });

  it("should serialize an empty array with a space parameter", () => {
    const value: never[] = [];
    expect(floatJsonSerializer(value, null, 2)).toBe("[]");
  });
});
