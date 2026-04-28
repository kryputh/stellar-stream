import { describe, it, expect, beforeAll, vi } from 'vitest';
import request from 'supertest';
import { app } from './backend/src/index';
import { Keypair, Transaction, Networks } from '@stellar/stellar-sdk';

describe('Integration: Auth Flow', () => {
    let clientKeypair: Keypair;
    let challengeTx: string;
    let validToken: string;

    beforeAll(() => {
        vi.stubEnv('JWT_SECRET', 'test_secret');
        clientKeypair = Keypair.random();
    });

    it('should generate a challenge', async () => {
        const response = await request(app)
            .get('/api/auth/challenge')
            .query({ accountId: clientKeypair.publicKey() });
        expect(response.status).toBe(200);
        challengeTx = response.body.transaction;
    });

    it('should verify with correct sig', async () => {
        const tx = new Transaction(challengeTx, Networks.TESTNET);
        tx.sign(clientKeypair);
        const response = await request(app)
            .post('/api/auth/token')
            .send({ transaction: tx.toXDR() });
        expect(response.status).toBe(200);
        validToken = response.body.token;
    });
});
