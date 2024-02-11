import { parseISODuration } from "./isoDuration";

describe("parseISODuration", () => {
  it("should parse a duration string with years, months, days, hours, minutes, and seconds", () => {
    expect(parseISODuration("P1Y2M3DT4H5M6S")).toBe(36561906);
  });

  it("should parse a duration string with years, months, days, hours, and minutes", () => {
    expect(parseISODuration("P1Y2M3DT4H5M")).toBe(36561900);
  });

  it("should parse a duration string with hours only", () => {
    expect(parseISODuration("PT4H")).toBe(14400);
  });

  it("should parse a duration string with days only", () => {
    expect(parseISODuration("P3D")).toBe(259200);
  });

  it("should parse a duration string with months only", () => {
    expect(parseISODuration("P2M")).toBe(5184000);
  });

  it("should parse a duration string with years only", () => {
    expect(parseISODuration("P1Y")).toBe(31104000);
  });

  it("should parse a duration string with seconds only", () => {
    expect(parseISODuration("PT6S")).toBe(6);
  });

  it("should parse a duration string with milliseconds", () => {
    expect(parseISODuration("PT6.789S")).toBe(6.789);
  });

  it("should parse a duration string with high precision", () => {
    expect(parseISODuration("PT488.799999S")).toBe(488.799999);
  });

  it("should parse a duration string with high precision with leading zeros", () => {
    expect(parseISODuration("PT12134.0000123S")).toBe(12134.0000123);
  });

  it("should parse a duration string with all components", () => {
    expect(parseISODuration("P1Y2M3DT4H5M6.789S")).toBe(36561906.789);
  });
});
