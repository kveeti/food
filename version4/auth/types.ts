export type AccessClaims = {
  issuer: string;
  subject: string;
  expiresAt: Date;
};

export type ProviderTokens = {
  accessToken: string;
  refreshToken: string;
  refreshExpiresAt: Date;
  accessClaims: AccessClaims;
};

export type OidcIdentity = {
  issuer: string;
  subject: string;
  email: string | null;
};

export type User = {
  id: string;
  email: string | null;
};

export type AppEnv = {
  Variables: {
    user: User;
  };
};
