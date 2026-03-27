const http = require('http');

// Test the snapshot endpoint
async function testSnapshotEndpoint() {
  const options = {
    hostname: 'localhost',
    port: 3001,
    path: '/api/streams/1/snapshot',
    method: 'GET',
    headers: {
      'Content-Type': 'application/json'
    }
  };

  return new Promise((resolve, reject) => {
    const req = http.request(options, (res) => {
      let data = '';
      
      res.on('data', (chunk) => {
        data += chunk;
      });
      
      res.on('end', () => {
        resolve({
          statusCode: res.statusCode,
          headers: res.headers,
          data: data
        });
      });
    });

    req.on('error', (err) => {
      reject(err);
    });

    req.end();
  });
}

// Test with non-existent stream
async function testNotFound() {
  const options = {
    hostname: 'localhost',
    port: 3001,
    path: '/api/streams/99999/snapshot',
    method: 'GET',
    headers: {
      'Content-Type': 'application/json'
    }
  };

  return new Promise((resolve, reject) => {
    const req = http.request(options, (res) => {
      let data = '';
      
      res.on('data', (chunk) => {
        data += chunk;
      });
      
      res.on('end', () => {
        resolve({
          statusCode: res.statusCode,
          data: data
        });
      });
    });

    req.on('error', (err) => {
      reject(err);
    });

    req.end();
  });
}

async function runTests() {
  console.log('Testing snapshot endpoint...');
  
  try {
    // Test 404 for non-existent stream
    console.log('\n1. Testing 404 for non-existent stream...');
    const notFoundResult = await testNotFound();
    console.log(`Status: ${notFoundResult.statusCode}`);
    console.log(`Response: ${notFoundResult.data}`);
    
    // Test existing stream (if server is running with data)
    console.log('\n2. Testing existing stream...');
    const result = await testSnapshotEndpoint();
    console.log(`Status: ${result.statusCode}`);
    
    if (result.statusCode === 200) {
      const response = JSON.parse(result.data);
      console.log('✅ Snapshot endpoint working!');
      console.log('Response structure:');
      console.log('- data.stream.id:', response.data?.stream?.id);
      console.log('- data.stream.progress.status:', response.data?.stream?.progress?.status);
      console.log('- data.history length:', response.data?.history?.length);
    } else {
      console.log(`Response: ${result.data}`);
    }
    
  } catch (error) {
    console.error('Error testing endpoint:', error.message);
    console.log('Make sure the server is running on localhost:3001');
  }
}

runTests();
