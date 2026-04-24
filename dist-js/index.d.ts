export declare type KeygenLicense = {
    key: string;
    code: string;
    detail: string;
    /**
     * License expiry can be null at the beginning (before activation)
     * when its policy's expirationBasis is *not* set to "FROM_CREATION".
     */
    expiry: string | null;
    valid: boolean;
    policyId: string;
    entitlements: string[];
    metadata: Record<string, any>;
};
export { KeygenError } from "./error";
export declare function getLicense(): Promise<KeygenLicense | null>;
export declare function getLicenseKey(): Promise<string | null>;
export declare function validateKey({ key, entitlements, cacheValidResponse, }: {
    key: string;
    entitlements?: string[];
    cacheValidResponse?: boolean;
}): Promise<KeygenLicense>;
export declare function validateCheckoutKey({ key, entitlements, ttlSeconds, ttlForever, }: {
    key: string;
    entitlements?: string[];
    ttlSeconds?: number;
    ttlForever?: boolean;
}): Promise<KeygenLicense>;
/**
 * Reset the local license state.
 *
 * @param remote When `true`, the plugin also releases the current machine
 *   slot on Keygen (DELETE machine by fingerprint) so a 1-of-1 license can
 *   be re-activated from a different device. Defaults to `false` (local-
 *   only) to preserve v2 behavior. Remote failures are swallowed so offline
 *   deactivation still works.
 */
export declare function resetLicense(opts?: {
    remote?: boolean;
}): Promise<void>;
export declare function resetLicenseKey(): Promise<void>;
