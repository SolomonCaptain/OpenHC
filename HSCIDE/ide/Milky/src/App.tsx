import { Suspense } from "react";
import { ClerkProvider } from "@clerk/clerk-react";
import { initClerk } from "tauri-plugin-clerk";
import { MUIThemeProvider } from "./theme/MUIThemeProvider";
import { AppRouter } from "./routes/AppRouter";
import { Box, CircularProgress, Typography } from "@mui/material";

// 初始化 Clerk（tauri-plugin-clerk 的核心方法）
// 这会修补 globalThis.fetch 以将 Clerk 调用路由到 Rust 后端
initClerk();

// 加载中组件
function LoadingScreen() {
  return (
    <Box
      sx={{
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 2,
      }}
    >
      <CircularProgress size={48} />
      <Typography variant="body1" color="text.secondary">
        正在加载...
      </Typography>
    </Box>
  );
}

function App() {
  return (
    <Suspense fallback={<LoadingScreen />}>
      <ClerkProvider
        publishableKey={import.meta.env.VITE_CLERK_PUBLISHABLE_KEY}
        signInUrl="/auth"
        signUpUrl="/auth"
        afterSignInUrl="/home"
        afterSignUpUrl="/home"
      >
        <MUIThemeProvider>
          <AppRouter />
        </MUIThemeProvider>
      </ClerkProvider>
    </Suspense>
  );
}

export default App;