require "socket"

total_bytes = 0
total_bytes_read = 0
total_sends = 0
Thread.new do
  loop do
    puts "total_bytes=#{total_bytes} / total_bytes_read=#{total_bytes_read} / total_sends=#{total_sends}"
    sleep 1
  end
end
THREADS = 200

threads = []
THREADS.times do |i|
  threads << Thread.new(i) do |i|
    rand(20).times do
      s = TCPSocket.new "127.0.0.1", 3333
      sleep rand
      if rand < 0.2
        rand(100).times do
          bytes = "foo" * rand(10)
          s.puts bytes
          sleep rand * 0.01
          total_bytes += bytes.length
          total_sends += 1
        end
      end
      begin
        while bytes = s.read_nonblock(65535)
          total_bytes_read += bytes.length
          break if rand < 0.5
        end
      rescue
      end
      sleep rand
      s.close if rand < 0.8
    end
    sleep 11
    puts "thread #{i} done"
  end
end
threads.each(&:join)
