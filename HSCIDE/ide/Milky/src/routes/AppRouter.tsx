import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { SignedIn, SignedOut } from "@clerk/clerk-react";
import { AuthPage } from "../pages/AuthPage";
import { HomePage } from "../pages/HomePage";

export function AppRouter() {
    return (
        <BrowserRouter>
            <Routes>
                {/* 公开路由 - 仅未登录用户可访问 */}
                <Route
                    path="/auth/*"
                    element={
                        <SignedOut>
                            <AuthPage />
                        </SignedOut>
                    }
                />

                {/* 受保护路由 - 仅登录用户可访问 */}
                <Route
                    path="/home"
                    element={
                        <SignedIn>
                            <HomePage />
                        </SignedIn>
                    }
                />

                {/* 根路径重定向 */}
                <Route
                    path="/"
                    element={
                        <>
                            <SignedIn>
                                <Navigate to="/home" replace />
                            </SignedIn>
                            <SignedOut>
                                <Navigate to="/auth" replace />
                            </SignedOut>
                        </>
                    }
                />

                {/* 404 路由 */}
                <Route
                    path="*"
                    element={
                        <>
                            <SignedIn>
                                <Navigate to="/home" replace />
                            </SignedIn>
                            <SignedOut>
                                <Navigate to="/auth" replace />
                            </SignedOut>
                        </>
                    }
                />
            </Routes>
        </BrowserRouter>
    );
}