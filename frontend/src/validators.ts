import {
  required,
  minLength,
  maxLength,
  minValue,
  number,
  regex,
  email,
} from "react-admin";

export const validateTitle = [required(), minLength(1), maxLength(100)];
export const validateDescription = [required(), minLength(1), maxLength(5000)];
export const validateAudioBitrate = [minValue(0), number()];
export const validateAudioTrackCount = [minValue(0), number()];
export const validateContentType = [
  required(),
  minLength(1),
  regex(/^[\w-]+\/[\w-]+$/, "Must be in the format 'type/subtype'"),
];
export const validateFilename = [required(), minLength(1), maxLength(255)];
export const validateFrameRate = [minValue(0), number()];
export const validateHeight = [minValue(0), number()];
export const validateWidth = [minValue(0), number()];
export const validateVideoBitrate = [minValue(0), number()];
export const validateSize = [minValue(0), number()];
export const validateLastModified = [];
export const validateEmail = [email()];
export const validateUri = [
  minLength(1),
  regex(/^.+:.+/, "Must be a valid URI"),
];
