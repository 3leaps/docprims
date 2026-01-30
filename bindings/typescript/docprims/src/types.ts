export type ExtractLimits = {
	max_input_bytes?: number;
	max_output_bytes?: number;
	max_blocks?: number;
};

export type ExtractOptions = {
	limits?: ExtractLimits;
};

export type DocprimsExtractV0 = {
	schema_id: string;
	schema_version: string;
	generator: { name: string; version: string };
	source: {
		uri: string;
		format: { family: string; kind: string };
		sha256?: string | null;
	};
	document: {
		quality: { status: "complete" | "partial"; reason?: string | null };
		text: string;
		blocks: Array<{
			id: string;
			kind: string;
			text: string;
			doc_text_range: { start_byte: number; end_byte: number };
			loc: {
				kind: string;
				container: {
					kind: "file" | "archive";
					path: string;
					part?: string | null;
				};
				hints: Record<string, unknown>;
			};
			children: string[];
			role?: string | null;
		}>;
		warnings: string[];
		metadata?: unknown;
	};
};
