export class Float {
  private readonly value: number;

  constructor(value: number) {
    this.value = value;
  }

  toString(): string {
    const str = (
      Math.round((this.value + Number.EPSILON) * 10000) / 10000
    ).toFixed(4);

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
