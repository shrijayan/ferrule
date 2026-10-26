# frozen_string_literal: true

module Acme
  module Resources
    class Widgets
      # @return [Acme::Resources::Widgets::Parts]
      attr_reader :parts

      # Creates a widget with the given name and parts. The SKU is assigned by the
      # server.
      #
      # @overload create(name:, color: nil, dimensions: nil, tags: nil, request_options: {})
      #
      # @param name [String] The widget's display name.
      #
      # @param color [Symbol, Acme::Models::WidgetCreateParams::Color]
      #
      # @param dimensions [Acme::Models::WidgetCreateParams::Dimensions]
      #
      # @param tags [Array<String>]
      #
      # @param request_options [Acme::RequestOptions, Hash{Symbol=>Object}, nil]
      #
      # @return [Acme::Models::Widget]
      #
      # @see Acme::Models::WidgetCreateParams
      def create(params)
        parsed, options = Acme::WidgetCreateParams.dump_request(params)
        @client.request(method: :post, path: "v1/widgets", body: parsed, model: Acme::Widget, options: options)
      end

      # Retrieve a widget
      #
      # @overload retrieve(widget_id, request_options: {})
      #
      # @param widget_id [String]
      # @param request_options [Acme::RequestOptions, Hash{Symbol=>Object}, nil]
      #
      # @return [Acme::Models::Widget]
      #
      # @see Acme::Models::WidgetRetrieveParams
      def retrieve(widget_id, params = {})
        @client.request(
          method: :get,
          path: ["v1/widgets/%1$s", widget_id],
          model: Acme::Widget,
          options: params[:request_options]
        )
      end

      # List widgets
      #
      # @overload list(colors: nil, created_after: nil, limit: nil, request_options: {})
      #
      # @param colors [Array<String>]
      # @param created_after [Time]
      # @param limit [Integer]
      # @param request_options [Acme::RequestOptions, Hash{Symbol=>Object}, nil]
      #
      # @return [Acme::Models::WidgetListResponse]
      #
      # @see Acme::Models::WidgetListParams
      def list(params = {})
        parsed, options = Acme::WidgetListParams.dump_request(params)
        query = Acme::Internal::Util.encode_query_params(parsed)
        @client.request(
          method: :get,
          path: "v1/widgets",
          query: query,
          model: Acme::Models::WidgetListResponse,
          options: options
        )
      end

      # Delete a widget
      #
      # @overload delete(widget_id, request_options: {})
      #
      # @param widget_id [String]
      # @param request_options [Acme::RequestOptions, Hash{Symbol=>Object}, nil]
      #
      # @return [nil]
      #
      # @see Acme::Models::WidgetDeleteParams
      def delete(widget_id, params = {})
        @client.request(
          method: :delete,
          path: ["v1/widgets/%1$s", widget_id],
          model: NilClass,
          options: params[:request_options]
        )
      end

      # @api private
      #
      # @param client [Acme::Client]
      def initialize(client:)
        @client = client
        @parts = Acme::Resources::Widgets::Parts.new(client: client)
      end
    end
  end
end
