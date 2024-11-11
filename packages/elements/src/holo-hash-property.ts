import {
	HoloHash,
	decodeHashFromBase64,
	encodeHashToBase64,
} from '@holochain/client';
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
): PropertyDeclaration<HoloHash | null, unknown> {
	return {
		attribute: attributeName,
		type: Object,
		hasChanged: (oldVal: HoloHash | undefined, newVal: HoloHash | undefined) =>
			oldVal?.toString() !== newVal?.toString(),
		converter: {
			fromAttribute: value =>
				value && value.length > 0 && decodeHashFromBase64(value),
			toAttribute: hash => hash && encodeHashToBase64(hash),
		},
		reflect: true,
	};
}

export function hashesProperty(
	attributeName: string,
): PropertyDeclaration<HoloHash[], unknown> {
	return {
		attribute: attributeName,
		type: Object,
		hasChanged: (oldVal: HoloHash[], newVal: HoloHash[]) =>
			oldVal?.toString() !== newVal?.toString(),
		converter: {
			fromAttribute: value => {
				if (!value) return [];
				const split = value.split(',');
				return split.map(decodeHashFromBase64);
			},
			toAttribute: (hash: HoloHash[]) =>
				hash && hash.map(encodeHashToBase64).join(','),
		},
		reflect: true,
	};
}
