export const DocprimsErrorCode = {
	Ok: 0,
	Usage: 64,
	DataInvalid: 60,
	ResourceLimit: 70,
	Io: 74,
	Internal: 1,
} as const;

export type DocprimsErrorCode =
	(typeof DocprimsErrorCode)[keyof typeof DocprimsErrorCode];

const errorCodeNames: Record<number, string> = Object.fromEntries(
	Object.entries(DocprimsErrorCode).map(([k, v]) => [v, k]),
);

export class DocprimsError extends Error {
	public readonly code: DocprimsErrorCode;
	public readonly codeName: string;

	constructor(code: DocprimsErrorCode, message: string) {
		super(message);
		this.name = "DocprimsError";
		this.code = code;
		this.codeName = errorCodeNames[code] ?? `Unknown(${code})`;
	}
}
