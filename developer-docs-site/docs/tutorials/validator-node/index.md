---
title: "AIT-2"
slug: "index"
disable_pagination: true
hide_right_sidebar: true
hide_table_of_contents: false
hide_title: false
thinner_content: true
no_pad_top: false
---

**PLACEHOLDER LANDING PAGE**

 <div class="timeline">
<div class="step">
    <div>
        <div class="circle">1</div>
    </div>
    <div>
        <div class="step-title">Cerebras SIF container</div>
        <div class="step-caption">The Cerebras <a class="reference external" href="https://sylabs.io/guides/3.7/user-guide/">Singularity</a> container (SIF) is installed on all the nodes, including the chief and the worker nodes, and on the network attached CS system. This container consists of the Cerebras Graph Compiler (CGC) and other necessary software. </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">2</div>
    </div>
    <div>
        <div class="step-title">Slurm orchestrator</div>
        <div class="step-caption">The orchestrator software <a class="reference external" href="https://slurm.schedmd.com/quickstart.html">Slurm</a> is installed and is running on all the CPU nodes: on the chief node and on all the worker nodes. The coordination between the CS system and the nodes in the CS cluster is performed by the orchestrator software Slurm. </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">3</div>
    </div>
    <div>
        <div class="step-title">Hostnames</div>
        <div class="step-caption">You have the hostnames of the chief and the worker nodes. You will login to the chief node and perform all your work on the chief node. You will need hostnames of the worker nodes for debugging. </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">4</div>
    </div>
    <div>
        <div class="step-title">IP address of CS system</div>
        <div class="step-caption">You have the IP address and the port number of the network attached CS system accelerator. You will pass this IP address and port number to the <code class="docutils literal notranslate"><span class="pre">--cs_ip</span></code> flag of your runtime scripts during compiling and running your models. </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">5</div>
    </div>
    <div>
        <div class="step-title">Login steps</div>
        <div class="step-caption">Steps to login to the chief node of the CS system cluster. Logging into the chief node is done by using <code class="docutils literal notranslate"><span class="pre">ssh</span></code>. </div>
    </div>
</div>
<div class="step">
    <div>
    <div class="circle">6</div>
    </div>
    <div>
    <div class="step-title">Done</div>
    </div>
    </div>
    </div>

<div>
<p class="card-section-h2">Setting up a node for AIT-2</p>

<div class="row row-cols-1 row-cols-md-3 g-4">
  <div class="col">
    <div class="card h-100">
    <h3 class="card-header">Step 1</h3>
      <div class="card-body d-flex flex-column">
        <h3 class="card-title">Install a node</h3>
        <p class="card-text">Pick your preferred method from below:</p>
        <ul class="list-group list-group-flush">
          <li class="list-group-item"><a href="https://aptos.dev/tutorials/validator-node/run-validator-node-using-source/" class="card-link">Using Aptos source</a></li>
          <li class="list-group-item"><a href="https://aptos.dev/tutorials/validator-node/run-validator-node-using-aws" class="card-link">Using AWS</a></li>
          <li class="list-group-item"><a href="https://aptos.dev/tutorials/validator-node/run-validator-node-using-gcp" class="card-link">Using GCP</a></li>
          <li class="list-group-item"><a href="https://aptos.dev/tutorials/validator-node/run-validator-node-using-docker" class="card-link">Using Docker</a></li>
          <li class="list-group-item"><a href="https://aptos.dev/tutorials/validator-node/run-validator-node-using-azure" class="card-link">Using Azure</a></li>
        </ul>
      </div>
    </div>
  </div>
  <div class="col">
    <div class="card h-100">
     <h3 class="card-header">Step 2</h3>
      <div class="card-body d-flex flex-column">
        <h3 class="card-title">Set the node in Test mode</h3>
        <p class="card-text">Make sure to set your node in the Test mode. See the instructions in Step 1.</p>     
      </div>
    </div>
  </div>
  <div class="col">
    <div class="card h-100">   
    <h3 class="card-header">Step 3</h3>
      <div class="card-body d-flex flex-column">
        <h3 class="card-title">Check the node health</h3>
        <p class="card-text">Visit the AIT-2 node health checker page and check your node's health status.</p>
      </div>
    </div>
  </div>
</div>
<br />
<div class="row row-cols-1 row-cols-md-3 g-4">
  <div class="col">
    <div class="card h-100">
    <h3 class="card-header">Step 4</h3>
      <div class="card-body d-flex flex-column">
        <h3 class="card-title">Enter your healthy node info on Discord</h3>
        <p class="card-text">Provide the following information:</p>
        <ul class="list-group list-group-flush">
          <li class="list-group-item">Account address</li>
          <li class="list-group-item">Public keys</li>
          <li class="list-group-item">Network addresses</li>
          <li class="list-group-item">FullNode details (optional)</li>
        </ul>
      </div>
    </div>
  </div>
  <div class="col">
    <div class="card h-100">
     <h3 class="card-header">Step 5</h3>
      <div class="card-body d-flex flex-column">
        <h3 class="card-title">Provide KYC info</h3>
        <p class="card-text">When Aptos confirms your node is healthy, you will be asked to complete the KYC process.</p>     
      </div>
    </div>
  </div>
  <div class="col">
    <div class="card h-100">   
    <h3 class="card-header">Step 6</h3>
      <div class="card-body d-flex flex-column">
        <h3 class="card-title">Selected?</h3>
        <p class="card-text">If your node is selected, then follow the next steps here.</p>
      </div>
    </div>
  </div>
</div>
</div>
<br />
<br />
