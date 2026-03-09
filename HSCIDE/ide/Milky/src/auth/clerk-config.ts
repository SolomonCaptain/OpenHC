import type { Clerk } from "@clerk/clerk-js";

// 从环境变量获取 Clerk 公钥
const clerkPubKey = import.meta.env.VITE_CLERK_PUBLIC_KEY;

if (!clerkPubKey) {
    throw new Error("Clerk 公钥未设置，请检查环境变量 VITE_CLERK_PUBLIC_KEY 是否正确配置");
}

export const clerkConfig = {
    publishableKey: clerkPubKey,
    // 认证成功后的重定向路径
    afterSignInUrl: "/",
    afterSignUpUrl: "/",
    // 登出后的重定向路径
    afterSignOutUrl: "/auth",
};

export type { Clerk };