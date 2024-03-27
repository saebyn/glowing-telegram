/**
 * This module provides a custom JSON serializer that serializes floating point
 * numbers to strings with a fixed number of decimal places.
 *
 * This is required because of DaVinci Resolve's OTIO importer. DaVinci Resolve
 * requires that certain fields in the OTIO file be serialized as numbers with
 * decimal places.
 *
 * The default JSON serializer in JavaScript serializes
 * numbers with a variable number of decimal places,  which can result in
 * values like `1` being serialized as `1`. This can cause problems when
 * generating JSON for OTIO files to be imported into DaVinci Resolve.
 */

export class Float {
  private readonly value: number;

  constructor(value: number) {
    this.value = value;
  }

  toString(): string {
    // Here I'm rounding the number to 4 decimal places just to
    // avoid floating point precision issues. This is not strictly
    // necessary for DaVinci Resolve, but it's my personal preference.
    const str = this.value.toFixed(4);

    // DaVinci Resolve requires that floating point numbers have a decimal
    // point and at least one digit after the decimal point.
    return str.endsWith(".0000")
      ? str.slice(0, -3)
      : str.replace(/0+$/, "").replace(/\.$/, "");
  }
}

export default function floatJsonSerializer(
  value: any,
  ignored: unknown = null,
  space: number = 0,
  depth: number = 1
): string {
  const indent = " ".repeat(space * depth) || " ";
  const newline = space ? "\n" : "";
  const newlineAndIndent = space ? newline + indent : "";
  const previousIndent =
    space && depth > 0 ? " ".repeat(space * (depth - 1)) : "";

  const recur = (v: any) => floatJsonSerializer(v, ignored, space, depth + 1);

  // if the value is a Float, return its string representation
  if (value instanceof Float) {
    return value.toString();
  }

  // if value is an array, serialize its elements
  if (Array.isArray(value)) {
    if (value.length === 0) {
      return "[]";
    }

    return `[${newlineAndIndent}${value
      .map(recur)
      .join(`,${newline}${indent}`)}${newline}${previousIndent}]`;
  }

  // if value is an object, serialize its properties
  if (typeof value === "object" && value !== null) {
    // if the object has no properties, return an empty object
    if (Object.keys(value).length === 0) {
      return "{}";
    }

    const serialized: string[] = [];
    for (const key in value) {
      if (Object.prototype.hasOwnProperty.call(value, key)) {
        serialized.push(`"${key}": ${recur(value[key])}`);
      }
    }
    return `{${newlineAndIndent}${serialized.join(
      `,${newline}${indent}`
    )}${newline}${previousIndent}}`;
  }

  return JSON.stringify(value);
}
