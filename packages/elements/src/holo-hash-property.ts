import { HoloHash, decodeHashFromBase64 } from '@holochain/client';
import { PropertyDeclaration } from 'lit';

export function hashState() {
	return {
		hasChanged: (oldVal: HoloHash, newVal: HoloHash) =>
			oldVal?.toString() !== newVal?.toString(),
	};
}

export function hashesState() {
	return {
		hasChanged: (oldVal: HoloHash[], newVal: HoloHash[]) =>
			oldVal?.toString() !== newVal?.toString(),
	};
}

export function hashProperty(
	attributeName: string,
): PropertyDeclaration<object | null, unknown> {
	return {
		attribute: attributeName,
		type: Object,
		hasChanged: (oldVal: HoloHash | undefined, newVal: HoloHash | undefined) =>
			oldVal?.toString() !== newVal?.toString(),
		converter: (s: string | undefined) =>
			s && s.length > 0 && decodeHashFromBase64(s),
	};
}

export function hashesProperty(
	attributeName: string,
): PropertyDeclaration<object | null, unknown> {
	return {
		attribute: attributeName,
		type: Object,
		hasChanged: (oldVal: HoloHash[], newVal: HoloHash[]) =>
			oldVal?.toString() !== newVal?.toString(),
		converter: (s: string | null) => {
			if (!s) return [];
			const split = s.split(',');
			return split.map(decodeHashFromBase64);
		},
	};
}
