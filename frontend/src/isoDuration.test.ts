import { parseIntoSeconds } from "./isoDuration";

describe("parseISODuration", () => {
  it("should parse a duration string with years, months, days, hours, minutes, and seconds", () => {
    expect(parseIntoSeconds("P1Y2M3DT4H5M6S")).toBe(36561906);
  });

  it("should parse a duration string with years, months, days, hours, and minutes", () => {
    expect(parseIntoSeconds("P1Y2M3DT4H5M")).toBe(36561900);
  });

  it("should parse a duration string with hours only", () => {
    expect(parseIntoSeconds("PT4H")).toBe(14400);
  });

  it("should parse a duration string with days only", () => {
    expect(parseIntoSeconds("P3D")).toBe(259200);
  });

  it("should parse a duration string with months only", () => {
    expect(parseIntoSeconds("P2M")).toBe(5184000);
  });

  it("should parse a duration string with years only", () => {
    expect(parseIntoSeconds("P1Y")).toBe(31104000);
  });

  it("should parse a duration string with seconds only", () => {
    expect(parseIntoSeconds("PT6S")).toBe(6);
  });

  it("should parse a duration string with milliseconds", () => {
    expect(parseIntoSeconds("PT6.789S")).toBe(6.789);
  });

  it("should parse a duration string with high precision", () => {
    expect(parseIntoSeconds("PT488.799999S")).toBe(488.799999);
  });

  it("should parse a duration string with high precision with leading zeros", () => {
    expect(parseIntoSeconds("PT12134.0000123S")).toBe(12134.0000123);
  });

  it("should parse a duration string with all components", () => {
    expect(parseIntoSeconds("P1Y2M3DT4H5M6.789S")).toBe(36561906.789);
  });
});
