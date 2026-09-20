import type { NextConfig } from "next";

const config: NextConfig = {
  output: process.env.NODE_ENV === "development" ? undefined : "export",
  ...(process.env.NODE_ENV === "development"
    ? {
        async rewrites() {
          return [
            {
              source: "/api/:path*",
              destination: `${process.env.RESEARCH_API_ORIGIN || "http://127.0.0.1:8080"}/api/:path*`,
            },
          ];
        },
      }
    : {}),
  trailingSlash: true,
  images: { unoptimized: true },
};

export default config;
