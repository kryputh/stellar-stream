import { Request, Response, NextFunction } from "express";
import crypto from "crypto";

declare global {
  namespace Express {
    interface Request {
      requestId?: string; // Unique ID for log correlation
    }
  }
}

export function requestLogger(req: Request, res: Response, next: NextFunction) {
  // ✅ STEP 1: Generate unique request ID for log correlation
  const requestId = crypto.randomUUID();
  req.requestId = requestId;

  // ✅ STEP 2: Track request start time
  const start = Date.now();

  // ✅ STEP 3: Log AFTER response is sent
  res.on("finish", () => {
    const duration = Date.now() - start;

    // ✅ Required log data
    const logEntry = {
      requestId,
      method: req.method,
      route: req.originalUrl,
      statusCode: res.statusCode,
      duration: `${duration}ms`,
    };

    // ✅ STEP 4: Make logs readable in development
    if (process.env.NODE_ENV === "production") {
      // Machine-readable (for logging systems)
      console.log(JSON.stringify(logEntry));
    } else {
      // Human-readable (for local dev)
      console.log(
        `${req.method} ${req.originalUrl} ${res.statusCode} - ${duration}ms | id=${requestId}`
      );
    }
  });

  // ✅ STEP 5: Continue request
  next();
}