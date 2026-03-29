import { describe, it, expect, beforeAll, vi } from 'vitest';
import express from 'express';
import request from 'supertest';
import jwt from 'jsonwebtoken';
import { authMiddleware, generateChallenge } from './services/auth';

const TEST_SECRET = "test_secret_for_vitest";

describe('Authentication Logic & Middleware', () => {
  const testAccountId = 'GBVWD767T7RMTN6Y5Z6X3B2Y2Z6X3B2Y2Z6X3B2Y2Z6X3B2Y2Z6X3B2Y';
  let app: express.Express;

  beforeAll(() => {
    vi.stubEnv('JWT_SECRET', TEST_SECRET);
    app = express();
    app.use(express.json());
    
    // Define a dummy protected route for testing the middleware in isolation
    app.get('/api/test-protected', authMiddleware, (req, res) => {
      res.status(200).json({ 
        message: 'Success', 
        user: (req as any).user 
      });
    });
  });

  describe('generateChallenge', () => {
    it('should generate a non-empty SEP-10 challenge transaction string', () => {
      const challenge = generateChallenge(testAccountId);
      expect(typeof challenge).toBe('string');
      expect(challenge.length).toBeGreaterThan(0);
    });
  });

  describe('authMiddleware', () => {
    it('should reject requests with missing Authorization header (401)', async () => {
      const response = await request(app).get('/api/test-protected');
      
      expect(response.status).toBe(401);
      expect(response.body).toEqual({
        error: "Missing or invalid authorization header.",
        code: "UNAUTHORIZED",
      });
    });

    it('should reject requests with invalid header format (401)', async () => {
      const response = await request(app)
        .get('/api/test-protected')
        .set('Authorization', 'Basic wrongformat');
      
      expect(response.status).toBe(401);
      expect(response.body.code).toBe('UNAUTHORIZED');
    });

    it('should reject requests with an invalid token (401)', async () => {
      const response = await request(app)
        .get('/api/test-protected')
        .set('Authorization', 'Bearer this.is.not.a.valid.token');
      
      expect(response.status).toBe(401);
      expect(response.body).toEqual({
        error: "Invalid or expired authorization token.",
        code: "UNAUTHORIZED",
      });
    });

    it('should reject requests with an expired token (401)', async () => {
      const expiredToken = jwt.sign(
        { accountId: testAccountId }, 
        TEST_SECRET, 
        { expiresIn: '-1h' }
      );

      const response = await request(app)
        .get('/api/test-protected')
        .set('Authorization', `Bearer ${expiredToken}`);
      
      expect(response.status).toBe(401);
      expect(response.body).toEqual({
        error: "Invalid or expired authorization token.",
        code: "UNAUTHORIZED",
      });
    });

    it('should allow requests with a valid token and attach accountId to req.user (200)', async () => {
      const token = jwt.sign({ accountId: testAccountId }, TEST_SECRET, { expiresIn: '1h' });

      const response = await request(app)
        .get('/api/test-protected')
        .set('Authorization', `Bearer ${token}`);
      
      expect(response.status).toBe(200);
      expect(response.body.user.accountId).toBe(testAccountId);
    });
  });
});