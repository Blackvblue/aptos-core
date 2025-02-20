# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'resolv'
require 'uri'
require 'maxmind/geoip2'
require 'httparty'
require 'logging/logs'

VerifyResult = Struct.new(:valid, :message)
MetricsResult = Struct.new(:ok, :version, :message)
MetricsJsonResult = Struct.new(:ok, :data, :message)
LocationResult = Struct.new(:ok, :message, :record)
IPResult = Struct.new(:ok, :ip, :message)

# @param [String] hostname
def normalize_hostname!(hostname)
  hostname.strip!
  hostname.downcase!
  hostname.delete_prefix! 'http://'
  hostname.delete_prefix! 'https://'
  hostname.delete_suffix! '/'
end

# @param [String] metrics
# @return MetricsResult
def extract_metrics(metrics)
  return MetricsResult.new(false, nil, 'Metrics result is empty') unless metrics.present?

  metrics.split("\n").each_entry do |metric|
    next if metric.start_with? '#'

    name, value = metric.split
    # aptos_consensus_last_committed_version 8299
    return MetricsResult.new(true, value.to_i, nil) if name == 'aptos_consensus_last_committed_version'
  end

  MetricsResult.new(false, nil, 'could not find `aptos_consensus_last_committed_version` metric')
end

# @param [String] metrics
# @return Array<Tuple<String, Numeric, Hash>>
def metrics_to_json(metrics)
  res = metrics.split("\n").map do |metric|
    next if metric.start_with? '#'

    metric_line_to_json(metric)
  end.compact
  MetricsJsonResult.new(true, res, nil)
end

# input: "aptos_consensus_block_tracing_bucket{stage=\"committed\",le=\"0.01\"} 0"
# output: key="aptos_consensus_block_tracing_bucket", value=0, params={"stage"=>"committed", "le"=>"0.01"}
# @param [String] line
# @return Tuple<String, Numeric, Hash>
def metric_line_to_json(line)
  first_space = line.index ' '
  first_paren = line.index '{'

  if first_paren.nil?
    key, value = line.split
    value = JSON.parse(value)
    params = {}
  else
    key = line[..(first_paren - 1)]
    value = JSON.parse(line[(first_space + 1)..])
    params = line[(first_paren + 1)..(first_space - 2)].split(',').to_h do |parm|
      parm.split('=').tap { |kv| kv[1] = JSON.parse(kv[1]) }
    end
  end
  [key, value, params]
end

module NodeHelper
  class NodeVerifier
    include Logging::Logs

    # @param [String] hostname
    # @param [Integer] metrics_port
    def initialize(hostname, metrics_port, http_api_port)
      normalize_hostname!(hostname)

      @hostname = hostname
      @metrics_port = metrics_port
      @http_api_port = http_api_port
      @ip = resolve_ip
    end

    # @return [IPResult] ip
    attr_reader :ip

    # @return IPResult
    def resolve_ip
      return IPResult.new(true, @hostname, nil) if @hostname =~ Resolv::IPv4::Regex

      resolved_ip = Resolv::DNS.open do |dns|
        dns.timeouts = 0.5
        dns.getaddress @hostname
      end
      IPResult.new(true, resolved_ip, nil)
    rescue StandardError => e
      IPResult.new(false, nil, "DNS error: #{e}")
    end

    # @return [LocationResult]
    def location
      return LocationResult(false, "Can not fetch location with no IP: #{@ip.message}", nil) unless @ip.ok

      client = MaxMind::GeoIP2::Client.new(
        account_id: ENV.fetch('MAXMIND_ACCOUNT_ID'),
        license_key: ENV.fetch('MAXMIND_LICENSE_KEY')
      )
      LocationResult.new(true, nil, client.insights(@ip.ip))
    rescue StandardError => e
      Sentry.capture_exception(e)
      LocationResult.new(false, "Error: #{e}", nil)
    end

    # @return MetricsResult
    def fetch_metrics
      yield HTTParty.get("http://#{@hostname}:#{@metrics_port}/metrics", open_timeout: 2, read_timeout: 3,
                                                                         max_retries: 0).body
    rescue Net::ReadTimeout => e
      MetricsResult.new(false, nil, "Read timeout: #{e}")
    rescue Net::OpenTimeout => e
      MetricsResult.new(false, nil, "Open timeout: #{e}")
    rescue StandardError => e
      log e.to_s
      MetricsResult.new(false, nil, "Error: #{e}")
    end

    # @return VerifyResult
    def verify_metrics
      res1 = fetch_metrics { |b| extract_metrics(b) }
      return VerifyResult.new(false, "Could not verify metrics; #{res1.message}") unless res1.ok

      # Sleep to allow their node to produce more versions
      sleep 1

      res2 = fetch_metrics { |b| extract_metrics(b) }
      return VerifyResult.new(false, "Could not verify metrics; #{res2.message}") unless res2.ok

      unless res2.version > res1.version
        return VerifyResult.new(false,
                                'Metrics last synced version did not increase. Ensure your node is running, and retry.')
      end

      VerifyResult.new(true, 'Metrics verified successfully!')
    end

    # @return [MetricsJsonResult]
    def fetch_json_metrics
      res = fetch_metrics { |b| metrics_to_json(b) }
      case res
      when MetricsResult
        # Wrap the error
        MetricsJsonResult.new(false, nil, res.message)
      when MetricsJsonResult
        res
      end
    end

    # @return [Array<VerifyResult>]
    def verify
      validations = []
      validations << verify_metrics
      validations
    end
  end
end
